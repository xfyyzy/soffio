use std::sync::Arc;

use apalis::{
    layers::WorkerBuilderExt,
    prelude::{Monitor, WorkerBuilder, WorkerFactoryFn},
};
use apalis_cron::CronStream;
use apalis_sql::{Config as ApalisSqlConfig, postgres::PostgresStorage};
use soffio::{
    application::admin::{
        audit::AdminAuditService,
        chrome::AdminChromeService,
        dashboard::{AdminDashboardDeps, AdminDashboardService},
        jobs::AdminJobService,
        navigation::AdminNavigationService,
        pages::AdminPageService,
        posts::AdminPostService,
        settings::AdminSettingsService,
        snapshots::AdminSnapshotService,
        tags::AdminTagService,
        uploads::AdminUploadService,
    },
    application::api_keys::ApiKeyService,
    application::chrome::ChromeService,
    application::error::AppError,
    application::feed::FeedService,
    application::jobs::{
        ExpireApiKeysContext, JobWorkerContext, expire_api_keys_schedule,
        process_expire_api_keys_job, process_publish_page_job, process_publish_post_job,
    },
    application::page::PageService,
    application::render::{
        InFlightRenders, RenderMailbox, process_render_page_job, process_render_post_job,
        render_service,
    },
    application::repos::{
        ApiKeysRepo, AuditRepo, JobsRepo, NavigationRepo, NavigationWriteRepo, PagesRepo,
        PagesWriteRepo, PostsRepo, PostsWriteRepo, SectionsRepo, SettingsRepo, SnapshotsRepo,
        TagsRepo, TagsWriteRepo, UploadsRepo,
    },
    application::sitemap::SitemapService,
    application::snapshot_preview::SnapshotPreviewService,
    application::syndication::SyndicationService,
    cache::{
        CacheConfig, CacheConsumer, CacheRegistry, CacheState, CacheTrigger, EventQueue, L0Store,
        L1Store,
    },
    config,
    domain::types::JobType,
    infra::{
        db::PostgresRepositories,
        error::InfraError,
        http::{self, AdminState, ApiState, HttpState, RouterState},
        uploads::UploadStorage,
    },
};
use tokio::try_join;
use tracing::error;

pub(super) struct ApplicationContext {
    pub(super) http_state: HttpState,
    pub(super) admin_state: AdminState,
    pub(super) api_state: ApiState,
    pub(super) job_context: JobWorkerContext,
    pub(super) api_keys: Arc<ApiKeyService>,
    pub(super) cache_trigger: Option<Arc<CacheTrigger>>,
}

fn build_site_services(
    repositories: &Arc<PostgresRepositories>,
    cache: Option<Arc<L0Store>>,
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
        cache.clone(),
    ));
    let page = Arc::new(PageService::new(pages_repo, cache.clone()));
    let chrome = Arc::new(ChromeService::new(navigation_repo, settings_repo, cache));

    (feed, page, chrome)
}

pub(super) async fn init_repositories(
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

pub(super) fn build_application_context(
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

    let job_posts_repo: Arc<dyn PostsRepo> = job_repositories.clone();
    let job_posts_write_repo: Arc<dyn PostsWriteRepo> = job_repositories.clone();
    let job_sections_repo: Arc<dyn SectionsRepo> = job_repositories.clone();
    let job_tags_repo: Arc<dyn TagsRepo> = job_repositories.clone();
    let job_settings_repo: Arc<dyn SettingsRepo> = job_repositories.clone();
    let job_pages_repo: Arc<dyn PagesRepo> = job_repositories.clone();
    let job_pages_write_repo: Arc<dyn PagesWriteRepo> = job_repositories.clone();
    let job_jobs_repo: Arc<dyn JobsRepo> = job_repositories.clone();

    let upload_storage = Arc::new(
        UploadStorage::new(settings.uploads.directory.clone())
            .map_err(|err| AppError::from(InfraError::Io(err)))?,
    );

    let cache_config = CacheConfig::from(&settings.cache);
    let (cache_trigger, cache_state, l0_cache) = if cache_config.is_enabled() {
        let l0 = Arc::new(L0Store::new(&cache_config));
        let l1 = Arc::new(L1Store::new(&cache_config));
        let registry = Arc::new(CacheRegistry::new());
        let queue = Arc::new(EventQueue::new_with_limit(cache_config.max_event_queue_len));
        let consumer = Arc::new(CacheConsumer::new(
            cache_config.clone(),
            l0,
            l1.clone(),
            registry.clone(),
            queue.clone(),
            http_repositories.clone(),
        ));
        let l0_cache = if cache_config.enable_l0_cache {
            Some(consumer.l0().clone())
        } else {
            None
        };
        let trigger = Some(Arc::new(CacheTrigger::new(
            cache_config.clone(),
            queue,
            consumer.clone(),
        )));
        let state = Some(CacheState {
            config: cache_config.clone(),
            l1,
            registry,
        });
        (trigger, state, l0_cache)
    } else {
        (None, None, None)
    };

    let (feed_service_http, page_service_http, chrome_service_http) =
        build_site_services(&http_repositories, l0_cache.clone());
    let (feed_service_jobs, page_service_jobs, chrome_service_jobs) =
        build_site_services(&job_repositories, None);

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
    let admin_tag_service = Arc::new(
        AdminTagService::new(
            tags_repo.clone(),
            tags_write_repo.clone(),
            audit_service.clone(),
        )
        .with_cache_trigger_opt(cache_trigger.clone()),
    );
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
    let api_key_service = Arc::new(
        ApiKeyService::new(api_keys_repo.clone()).with_cache_trigger_opt(cache_trigger.clone()),
    );

    let job_audit_service = AdminAuditService::new(job_repositories.clone());
    let job_admin_post_service = Arc::new(
        AdminPostService::new(
            job_posts_repo.clone(),
            job_posts_write_repo.clone(),
            job_sections_repo.clone(),
            job_jobs_repo.clone(),
            job_tags_repo.clone(),
            job_audit_service.clone(),
        )
        .with_cache_trigger_opt(cache_trigger.clone()),
    );
    let job_admin_page_service = Arc::new(
        AdminPageService::new(
            job_pages_repo.clone(),
            job_pages_write_repo.clone(),
            job_jobs_repo.clone(),
            job_audit_service.clone(),
            job_settings_repo.clone(),
        )
        .with_cache_trigger_opt(cache_trigger.clone()),
    );

    let syndication_service = Arc::new(SyndicationService::new(
        posts_repo.clone(),
        settings_repo.clone(),
        l0_cache.clone(),
    ));
    let sitemap_service = Arc::new(SitemapService::new(
        posts_repo.clone(),
        pages_repo.clone(),
        settings_repo.clone(),
        l0_cache.clone(),
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
        dashboard: Arc::new(AdminDashboardService::new(AdminDashboardDeps {
            posts: posts_repo.clone(),
            pages: pages_repo.clone(),
            tags: tags_repo.clone(),
            navigation: navigation_repo.clone(),
            uploads: uploads_repo.clone(),
            api_keys: api_keys_repo.clone(),
        })),
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
        admin_posts: job_admin_post_service,
        admin_pages: job_admin_page_service,
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

pub(super) async fn run_serve(settings: config::Settings) -> Result<(), AppError> {
    let (http_repositories, job_repositories) = init_repositories(&settings).await?;
    let app = build_application_context(
        http_repositories.clone(),
        job_repositories.clone(),
        &settings,
    )?;

    if let Some(trigger) = &app.cache_trigger {
        trigger.warmup_on_startup().await;
    }

    let cache_handle = if let Some(trigger) = app.cache_trigger.clone() {
        let interval_ms = trigger.config().auto_consume_interval_ms;
        Some(tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_millis(interval_ms));
            interval.tick().await;
            loop {
                interval.tick().await;
                trigger.consumer().consume_full().await;
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
