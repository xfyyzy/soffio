use std::{
    process,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};

use apalis::{
    layers::WorkerBuilderExt,
    prelude::{Data, Monitor, WorkerBuilder, WorkerFactoryFn},
};
use apalis_cron::CronStream;
use apalis_sql::{Config as ApalisSqlConfig, postgres::PostgresStorage};
use futures::stream::TryStreamExt;
use soffio::{
    application::error::AppError,
    application::{
        admin::{
            audit::AdminAuditService, chrome::AdminChromeService, dashboard::AdminDashboardService,
            jobs::AdminJobService, navigation::AdminNavigationService, pages::AdminPageService,
            posts::AdminPostService, settings::AdminSettingsService,
            snapshots::AdminSnapshotService, tags::AdminTagService, uploads::AdminUploadService,
        },
        api_keys::ApiKeyService,
        chrome::ChromeService,
        feed::FeedService,
        jobs::{
            ExpireApiKeysContext, JobWorkerContext, expire_api_keys_schedule,
            process_expire_api_keys_job, process_publish_page_job, process_publish_post_job,
        },
        page::PageService,
        render::{
            InFlightRenders, RenderMailbox, RenderPageJobPayload, RenderPipelineConfig,
            RenderPostJobPayload, configure_render_service, process_render_page_job,
            process_render_post_job, render_service,
        },
        repos::{
            ApiKeysRepo, AuditRepo, JobsRepo, NavigationRepo, NavigationWriteRepo, PagesRepo,
            PagesWriteRepo, PostsRepo, PostsWriteRepo, SectionsRepo, SettingsRepo, SnapshotsRepo,
            TagsRepo, TagsWriteRepo, UploadsRepo,
        },
        site,
        sitemap::SitemapService,
        snapshot_preview::SnapshotPreviewService,
        syndication::SyndicationService,
    },
    cache::{
        CacheConfig, CacheConsumer, CacheRegistry, CacheState, CacheTrigger, EventQueue, L0Store,
        L1Store,
    },
    config,
    domain::entities::{PageRecord, PostRecord},
    domain::types::JobType,
    infra::{
        db::PostgresRepositories,
        error::InfraError,
        http::{self, AdminState, ApiState, HttpState, RouterState},
        telemetry,
        uploads::UploadStorage,
    },
};
use tokio::try_join;
use tracing::{Dispatch, Level, dispatcher, error, info};
use tracing_subscriber::fmt as tracing_fmt;

mod migrations_tool;

#[tokio::main]
async fn main() {
    if let Err(error) = run().await {
        report_application_error(&error);
        process::exit(1);
    }
}

fn report_application_error(error: &AppError) {
    if dispatcher::has_been_set() {
        error!(error = %error, "application error");
        return;
    }

    let subscriber = tracing_fmt().with_max_level(Level::ERROR).finish();
    let dispatch = Dispatch::new(subscriber);
    dispatcher::with_default(&dispatch, || {
        error!(error = %error, "application error");
    });
}

async fn run() -> Result<(), AppError> {
    let (cli_args, settings) = config::load_with_cli()
        .map_err(|err| AppError::unexpected(format!("failed to load configuration: {err}")))?;

    let command = cli_args
        .command
        .unwrap_or(config::Command::Serve(Box::<config::ServeArgs>::default()));

    telemetry::init(&settings.logging).map_err(AppError::from)?;
    configure_render_service(RenderPipelineConfig::from(&settings.render))
        .map_err(|err| AppError::unexpected(err.to_string()))?;

    match command {
        config::Command::Serve(_) => run_serve(settings).await,
        config::Command::RenderAll(args) => run_renderall(settings, args).await,
        config::Command::ExportSite(args) => run_export_site(settings, args).await,
        config::Command::ImportSite(args) => run_import_site(settings, args).await,
        config::Command::Migrations(args) => run_migrations(settings, args).await,
    }
}

async fn run_serve(settings: config::Settings) -> Result<(), AppError> {
    let (http_repositories, job_repositories) = init_repositories(&settings).await?;
    let app = build_application_context(
        http_repositories.clone(),
        job_repositories.clone(),
        &settings,
    )?;

    // Perform startup cache warmup (queues event for async consumption)
    if let Some(trigger) = &app.cache_trigger {
        trigger.warmup_on_startup().await;
    }

    // Spawn cache auto-consume timer
    let cache_handle = if let Some(trigger) = app.cache_trigger.clone() {
        let interval_ms = trigger.config().auto_consume_interval_ms;
        Some(tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_millis(interval_ms));
            interval.tick().await; // Skip the first immediate tick
            loop {
                interval.tick().await;
                trigger.consumer().consume().await;
            }
        }))
    } else {
        None
    };

    let monitor_handle = spawn_job_monitor(
        job_repositories,
        app.job_context.clone(),
        app.api_keys.clone(),
        &settings.jobs,
    );

    let result = serve_http(&settings, app.http_state, app.admin_state, app.api_state).await;

    monitor_handle.abort();
    let _ = monitor_handle.await;

    if let Some(h) = cache_handle {
        h.abort();
        let _ = h.await;
    }

    result
}

async fn run_renderall(
    settings: config::Settings,
    args: config::RenderAllArgs,
) -> Result<(), AppError> {
    let (http_repositories, job_repositories) = init_repositories(&settings).await?;
    let app = build_application_context(http_repositories, job_repositories, &settings)?;
    let job_context = app.job_context;

    let filter_specified = args.posts || args.pages;
    let render_posts = if filter_specified { args.posts } else { true };
    let render_pages = if filter_specified { args.pages } else { true };

    if !render_posts && !render_pages {
        return Err(AppError::validation(
            "renderall requires at least one of --posts or --pages",
        ));
    }

    let concurrency = args.concurrency.clamp(1, 32);

    info!(
        target = "soffio::renderall",
        concurrency,
        posts = render_posts,
        pages = render_pages,
        "Starting renderall"
    );

    if render_posts {
        render_all_posts(&job_context, concurrency).await?;
    }

    if render_pages {
        render_all_pages(&job_context, concurrency).await?;
    }

    Ok(())
}

async fn run_export_site(
    settings: config::Settings,
    args: config::ExportArgs,
) -> Result<(), AppError> {
    let (http_repositories, _) = init_repositories(&settings).await?;
    let path = args.file;

    info!(
        target = "soffio::export",
        path = %path.display(),
        "Starting export"
    );

    site::export_site(&http_repositories, &path).await?;
    info!(target = "soffio::export", "Export completed");
    Ok(())
}

async fn run_import_site(
    settings: config::Settings,
    args: config::ImportArgs,
) -> Result<(), AppError> {
    let (http_repositories, _) = init_repositories(&settings).await?;
    let path = args.file;

    info!(
        target = "soffio::import",
        path = %path.display(),
        "Starting import"
    );

    site::import_site(&http_repositories, &path).await?;
    info!(
        target = "soffio::import",
        "Import completed. Re-run renderall to regenerate derived content."
    );
    Ok(())
}

async fn run_migrations(
    settings: config::Settings,
    args: config::MigrationsArgs,
) -> Result<(), AppError> {
    match args.command {
        config::MigrationsCommand::Reconcile(cmd) => {
            migrations_tool::reconcile_archive(&settings.database, &cmd).await?
        }
    }
    Ok(())
}

struct ApplicationContext {
    http_state: HttpState,
    admin_state: AdminState,
    api_state: ApiState,
    job_context: JobWorkerContext,
    api_keys: Arc<ApiKeyService>,
    cache_trigger: Option<Arc<CacheTrigger>>,
}

fn build_site_services(
    repositories: &Arc<PostgresRepositories>,
) -> (Arc<FeedService>, Arc<PageService>, Arc<ChromeService>) {
    let posts_repo: Arc<dyn PostsRepo> = repositories.clone();
    let sections_repo: Arc<dyn SectionsRepo> = repositories.clone();
    let tags_repo: Arc<dyn TagsRepo> = repositories.clone();
    let settings_repo: Arc<dyn SettingsRepo> = repositories.clone();
    let navigation_repo: Arc<dyn NavigationRepo> = repositories.clone();
    let pages_repo: Arc<dyn PagesRepo> = repositories.clone();

    let feed = Arc::new(FeedService::new(
        posts_repo,
        sections_repo,
        tags_repo,
        settings_repo.clone(),
    ));
    let page = Arc::new(PageService::new(pages_repo));
    let chrome = Arc::new(ChromeService::new(navigation_repo, settings_repo));

    (feed, page, chrome)
}

async fn init_repositories(
    settings: &config::Settings,
) -> Result<(Arc<PostgresRepositories>, Arc<PostgresRepositories>), AppError> {
    let database_url = settings
        .database
        .url
        .as_ref()
        .ok_or_else(|| InfraError::configuration("database url is not configured"))
        .map_err(AppError::from)?;

    let http_pool =
        PostgresRepositories::connect(database_url, settings.database.http_max_connections.get())
            .await
            .map_err(|err| AppError::from(InfraError::database(err.to_string())))?;

    PostgresRepositories::run_migrations(&http_pool)
        .await
        .map_err(|err| AppError::from(InfraError::database(err.to_string())))?;

    let jobs_pool =
        PostgresRepositories::connect(database_url, settings.database.jobs_max_connections.get())
            .await
            .map_err(|err| AppError::from(InfraError::database(err.to_string())))?;

    Ok((
        Arc::new(PostgresRepositories::new(http_pool)),
        Arc::new(PostgresRepositories::new(jobs_pool)),
    ))
}

fn build_application_context(
    http_repositories: Arc<PostgresRepositories>,
    job_repositories: Arc<PostgresRepositories>,
    settings: &config::Settings,
) -> Result<ApplicationContext, AppError> {
    let posts_repo: Arc<dyn PostsRepo> = http_repositories.clone();
    let posts_write_repo: Arc<dyn PostsWriteRepo> = http_repositories.clone();
    let sections_repo: Arc<dyn SectionsRepo> = http_repositories.clone();
    let tags_repo: Arc<dyn TagsRepo> = http_repositories.clone();
    let tags_write_repo: Arc<dyn TagsWriteRepo> = http_repositories.clone();
    let settings_repo: Arc<dyn SettingsRepo> = http_repositories.clone();
    let navigation_repo: Arc<dyn NavigationRepo> = http_repositories.clone();
    let navigation_write_repo: Arc<dyn NavigationWriteRepo> = http_repositories.clone();
    let pages_repo: Arc<dyn PagesRepo> = http_repositories.clone();
    let pages_write_repo: Arc<dyn PagesWriteRepo> = http_repositories.clone();
    let uploads_repo: Arc<dyn UploadsRepo> = http_repositories.clone();
    let api_keys_repo: Arc<dyn ApiKeysRepo> = http_repositories.clone();
    let audit_repo: Arc<dyn AuditRepo> = http_repositories.clone();
    let jobs_repo: Arc<dyn JobsRepo> = http_repositories.clone();
    let snapshots_repo: Arc<dyn SnapshotsRepo> = http_repositories.clone();

    let (feed_service_http, page_service_http, chrome_service_http) =
        build_site_services(&http_repositories);
    let (feed_service_jobs, page_service_jobs, chrome_service_jobs) =
        build_site_services(&job_repositories);

    let upload_storage = Arc::new(
        UploadStorage::new(settings.uploads.directory.clone())
            .map_err(|err| AppError::from(InfraError::Io(err)))?,
    );

    // Initialize cache infrastructure
    let cache_config = CacheConfig::from(&settings.cache);
    let (cache_trigger, cache_state) = if cache_config.is_enabled() {
        let l0 = Arc::new(L0Store::new(&cache_config));
        let l1 = Arc::new(L1Store::new(&cache_config));
        let registry = Arc::new(CacheRegistry::new());
        let queue = Arc::new(EventQueue::new());
        let consumer = Arc::new(CacheConsumer::new(
            cache_config.clone(),
            l0,
            l1.clone(),
            registry.clone(),
            queue.clone(),
            http_repositories.clone(),
        ));
        let trigger = Some(Arc::new(CacheTrigger::new(
            cache_config.clone(),
            queue,
            consumer,
        )));
        let state = Some(CacheState {
            config: cache_config,
            l1,
            registry,
        });
        (trigger, state)
    } else {
        (None, None)
    };

    let audit_service = AdminAuditService::new(audit_repo.clone());
    let admin_post_service = Arc::new(
        AdminPostService::new(
            posts_repo.clone(),
            posts_write_repo.clone(),
            sections_repo.clone(),
            jobs_repo.clone(),
            tags_repo.clone(),
            audit_service.clone(),
        )
        .with_cache_trigger_opt(cache_trigger.clone()),
    );
    let admin_page_service = Arc::new(
        AdminPageService::new(
            pages_repo.clone(),
            pages_write_repo.clone(),
            jobs_repo.clone(),
            audit_service.clone(),
            settings_repo.clone(),
        )
        .with_cache_trigger_opt(cache_trigger.clone()),
    );
    let admin_tag_service = Arc::new(AdminTagService::new(
        tags_repo.clone(),
        tags_write_repo.clone(),
        audit_service.clone(),
    ));
    let admin_navigation_service = Arc::new(
        AdminNavigationService::new(
            navigation_repo.clone(),
            navigation_write_repo.clone(),
            pages_repo.clone(),
            audit_service.clone(),
        )
        .with_cache_trigger_opt(cache_trigger.clone()),
    );
    let admin_settings_service = Arc::new(
        AdminSettingsService::new(settings_repo.clone(), audit_service.clone())
            .with_cache_trigger_opt(cache_trigger.clone()),
    );
    let admin_upload_service = Arc::new(AdminUploadService::new(
        uploads_repo.clone(),
        audit_service.clone(),
    ));
    let admin_job_service = Arc::new(AdminJobService::new(
        jobs_repo.clone(),
        audit_service.clone(),
    ));
    let admin_snapshot_service = Arc::new(AdminSnapshotService::new(snapshots_repo.clone()));
    let snapshot_preview_service = Arc::new(SnapshotPreviewService::new(
        snapshots_repo.clone(),
        tags_repo.clone(),
        settings_repo.clone(),
    ));
    let admin_audit_service = Arc::new(audit_service);
    let api_key_service = Arc::new(ApiKeyService::new(api_keys_repo.clone()));

    let syndication_service = Arc::new(SyndicationService::new(
        posts_repo.clone(),
        settings_repo.clone(),
    ));
    let sitemap_service = Arc::new(SitemapService::new(
        posts_repo.clone(),
        pages_repo.clone(),
        settings_repo.clone(),
    ));

    let http_state = HttpState {
        feed: feed_service_http.clone(),
        pages: page_service_http.clone(),
        chrome: chrome_service_http.clone(),
        syndication: syndication_service,
        sitemap: sitemap_service,
        db: http_repositories.clone(),
        upload_storage: upload_storage.clone(),
        snapshot_preview: snapshot_preview_service.clone(),
        cache: cache_state,
    };

    let admin_state = AdminState {
        db: http_repositories.clone(),
        chrome: Arc::new(AdminChromeService::new(settings_repo.clone())),
        dashboard: Arc::new(AdminDashboardService::new(
            posts_repo.clone(),
            pages_repo.clone(),
            tags_repo.clone(),
            navigation_repo.clone(),
            uploads_repo.clone(),
            api_keys_repo.clone(),
        )),
        posts: admin_post_service,
        pages: admin_page_service,
        tags: admin_tag_service,
        navigation: admin_navigation_service,
        settings: admin_settings_service,
        uploads: admin_upload_service,
        upload_storage: upload_storage.clone(),
        upload_limit_bytes: settings.uploads.max_request_bytes.get(),
        jobs: admin_job_service,
        audit: admin_audit_service,
        api_keys: api_key_service.clone(),
        snapshots: admin_snapshot_service.clone(),
    };

    let rate_limiter = Arc::new(http::ApiRateLimiter::new(
        std::time::Duration::from_secs(settings.api_rate_limit.window_seconds.get() as u64),
        settings.api_rate_limit.max_requests.get(),
    ));

    let api_state = ApiState {
        api_keys: admin_state.api_keys.clone(),
        posts: admin_state.posts.clone(),
        pages: admin_state.pages.clone(),
        tags: admin_state.tags.clone(),
        navigation: admin_state.navigation.clone(),
        uploads: admin_state.uploads.clone(),
        settings: admin_state.settings.clone(),
        jobs: admin_state.jobs.clone(),
        audit: admin_state.audit.clone(),
        snapshots: admin_snapshot_service.clone(),
        db: http_repositories.clone(),
        upload_storage: upload_storage.clone(),
        rate_limiter,
    };

    let render_mailbox = RenderMailbox::new();
    let inflight_renders = InFlightRenders::new();

    let job_context = JobWorkerContext {
        repositories: job_repositories,
        renderer: render_service(),
        feed: feed_service_jobs,
        pages: page_service_jobs,
        snapshot_preview: snapshot_preview_service.clone(),
        chrome: chrome_service_jobs,
        upload_storage,
        render_mailbox,
        inflight_renders,
    };

    Ok(ApplicationContext {
        http_state,
        admin_state,
        api_state,
        job_context,
        api_keys: api_key_service,
        cache_trigger,
    })
}

async fn render_all_posts(ctx: &JobWorkerContext, concurrency: usize) -> Result<(), AppError> {
    let total = Arc::new(AtomicUsize::new(0));
    let worker_ctx = ctx.clone();
    let total_handle = total.clone();

    ctx.repositories
        .stream_all_posts()
        .map_err(|err| AppError::unexpected(err.to_string()))
        .try_for_each_concurrent(Some(concurrency), move |post| {
            let ctx = worker_ctx.clone();
            let counter = total_handle.clone();
            async move {
                render_post(&ctx, post).await?;
                counter.fetch_add(1, Ordering::Relaxed);
                Ok(())
            }
        })
        .await?;

    let count = total.load(Ordering::Relaxed);
    info!(
        target = "soffio::renderall",
        posts = count,
        "Rendered all posts"
    );
    Ok(())
}

async fn render_all_pages(ctx: &JobWorkerContext, concurrency: usize) -> Result<(), AppError> {
    let total = Arc::new(AtomicUsize::new(0));
    let worker_ctx = ctx.clone();
    let total_handle = total.clone();

    ctx.repositories
        .stream_all_pages()
        .map_err(|err| AppError::unexpected(err.to_string()))
        .try_for_each_concurrent(Some(concurrency), move |page| {
            let ctx = worker_ctx.clone();
            let counter = total_handle.clone();
            async move {
                render_page(&ctx, page).await?;
                counter.fetch_add(1, Ordering::Relaxed);
                Ok(())
            }
        })
        .await?;

    let count = total.load(Ordering::Relaxed);
    info!(
        target = "soffio::renderall",
        pages = count,
        "Rendered all pages"
    );
    Ok(())
}

async fn render_post(ctx: &JobWorkerContext, post: PostRecord) -> Result<(), AppError> {
    process_render_post_job(
        RenderPostJobPayload {
            slug: post.slug.clone(),
            body_markdown: post.body_markdown.clone(),
            summary_markdown: post.summary_markdown.clone(),
        },
        Data::new(ctx.clone()),
    )
    .await
    .map_err(|err| AppError::unexpected(format!("render `{}` failed: {err}", post.slug)))?;

    Ok(())
}

async fn render_page(ctx: &JobWorkerContext, page: PageRecord) -> Result<(), AppError> {
    process_render_page_job(
        RenderPageJobPayload {
            slug: page.slug.clone(),
            markdown: page.body_markdown.clone(),
        },
        Data::new(ctx.clone()),
    )
    .await
    .map_err(|err| AppError::unexpected(format!("render page `{}` failed: {err}", page.slug)))?;

    Ok(())
}

fn spawn_job_monitor(
    repositories: Arc<PostgresRepositories>,
    context: JobWorkerContext,
    api_keys: Arc<ApiKeyService>,
    jobs: &config::JobsSettings,
) -> tokio::task::JoinHandle<()> {
    let render_storage = PostgresStorage::new_with_config(
        repositories.pool().clone(),
        ApalisSqlConfig::new(JobType::RenderPost.as_str()),
    );
    let render_page_storage = PostgresStorage::new_with_config(
        repositories.pool().clone(),
        ApalisSqlConfig::new(JobType::RenderPage.as_str()),
    );
    let publish_post_storage = PostgresStorage::new_with_config(
        repositories.pool().clone(),
        ApalisSqlConfig::new(JobType::PublishPost.as_str()),
    );
    let publish_page_storage = PostgresStorage::new_with_config(
        repositories.pool().clone(),
        ApalisSqlConfig::new(JobType::PublishPage.as_str()),
    );

    let render_post_concurrency = jobs.render_post_concurrency.get() as usize;
    let render_page_concurrency = jobs.render_page_concurrency.get() as usize;
    let publish_post_concurrency = jobs.publish_post_concurrency.get() as usize;
    let publish_page_concurrency = jobs.publish_page_concurrency.get() as usize;

    let render_post_worker = WorkerBuilder::new("render-post-worker")
        .concurrency(render_post_concurrency)
        .data(context.clone())
        .backend(render_storage)
        .build_fn(process_render_post_job);
    let render_page_worker = WorkerBuilder::new("render-page-worker")
        .concurrency(render_page_concurrency)
        .data(context.clone())
        .backend(render_page_storage)
        .build_fn(process_render_page_job);
    let publish_post_worker = WorkerBuilder::new("publish-post-worker")
        .concurrency(publish_post_concurrency)
        .data(context.clone())
        .backend(publish_post_storage)
        .build_fn(process_publish_post_job);
    let publish_page_worker = WorkerBuilder::new("publish-page-worker")
        .concurrency(publish_page_concurrency)
        .data(context.clone())
        .backend(publish_page_storage)
        .build_fn(process_publish_page_job);

    // Cron-based API key expiration worker (runs hourly)
    let expire_api_keys_ctx = ExpireApiKeysContext { api_keys };
    let expire_api_keys_worker = WorkerBuilder::new("expire-api-keys-worker")
        .data(expire_api_keys_ctx)
        .backend(CronStream::new(expire_api_keys_schedule()))
        .build_fn(process_expire_api_keys_job);

    let monitor = Monitor::new()
        .register(render_post_worker)
        .register(render_page_worker)
        .register(publish_post_worker)
        .register(publish_page_worker)
        .register(expire_api_keys_worker);

    tokio::spawn(async move {
        if let Err(err) = monitor.run().await {
            error!(error = %err, "job monitor stopped");
        }
    })
}

async fn serve_http(
    settings: &config::Settings,
    http_state: HttpState,
    admin_state: AdminState,
    api_state: ApiState,
) -> Result<(), AppError> {
    let router_state = RouterState {
        http: http_state,
        api: api_state,
    };
    let public_router = http::build_router(router_state.clone());
    let upload_body_limit = settings.uploads.max_request_bytes.get() as usize;
    let admin_router = http::build_admin_router(admin_state, upload_body_limit);
    let api_router = http::build_api_v1_router(router_state.clone());

    let public_router = public_router
        .merge(api_router)
        .with_state(router_state.clone());

    let public_listener = tokio::net::TcpListener::bind(settings.server.public_addr)
        .await
        .map_err(|err| AppError::from(InfraError::from(err)))?;
    let admin_listener = tokio::net::TcpListener::bind(settings.server.admin_addr)
        .await
        .map_err(|err| AppError::from(InfraError::from(err)))?;

    let public_server = axum::serve(public_listener, public_router.into_make_service());
    let admin_server = axum::serve(admin_listener, admin_router.into_make_service());

    try_join!(public_server, admin_server)
        .map_err(|err| AppError::unexpected(format!("server error: {err}")))?;

    Ok(())
}

#[cfg(test)]
mod tests {}
