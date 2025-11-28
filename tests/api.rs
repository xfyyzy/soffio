use axum::extract::{Extension, Json, Query, State};
use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use sqlx::PgPool;
use time::OffsetDateTime;
use tokio::sync::Mutex;

use soffio::application::admin::audit::AdminAuditService;
use soffio::application::admin::navigation::AdminNavigationService;
use soffio::application::admin::pages::AdminPageService;
use soffio::application::admin::posts::AdminPostService;
use soffio::application::admin::settings::AdminSettingsService;
use soffio::application::admin::tags::AdminTagService;
use soffio::application::admin::uploads::AdminUploadService;
use soffio::application::api_keys::{ApiKeyService, IssueApiKeyCommand};
use soffio::application::chrome::ChromeService;
use soffio::application::feed::FeedService;
use soffio::application::page::PageService;
use soffio::application::pagination::{CursorPage, JobCursor, PageRequest};
use soffio::application::repos::{
    ApiKeysRepo, AuditRepo, JobQueryFilter, JobsRepo, NavigationRepo, NavigationWriteRepo,
    NewJobRecord, PagesRepo, PagesWriteRepo, PostsRepo, PostsWriteRepo, RepoError, SectionsRepo,
    SettingsRepo, TagsRepo, TagsWriteRepo, UpdateJobStateParams, UploadsRepo,
};
use soffio::domain::api_keys::ApiScope;
use soffio::domain::entities::JobRecord;
use soffio::domain::types::JobState;
use soffio::infra::cache::ResponseCache;
use soffio::infra::db::PostgresRepositories;
use soffio::infra::http::api::handlers;
use soffio::infra::http::api::models::*;
use soffio::infra::http::api::state::ApiState;
use soffio::infra::uploads::UploadStorage;

#[derive(Default)]
struct ImmediateJobsRepo {
    jobs: Mutex<HashMap<String, JobRecord>>,
}

#[async_trait]
impl JobsRepo for ImmediateJobsRepo {
    async fn enqueue_job(&self, job: NewJobRecord) -> Result<String, RepoError> {
        let id = uuid::Uuid::new_v4().to_string();
        let record = JobRecord {
            id: id.clone(),
            job_type: job.job_type,
            payload: job.payload,
            state: JobState::Done,
            attempts: 1,
            max_attempts: job.max_attempts,
            run_at: job.run_at,
            lock_at: None,
            lock_by: None,
            done_at: Some(OffsetDateTime::now_utc()),
            last_error: None,
            priority: job.priority,
        };
        self.jobs.lock().await.insert(id.clone(), record);
        Ok(id)
    }

    async fn update_job_state(&self, params: UpdateJobStateParams) -> Result<(), RepoError> {
        let mut jobs = self.jobs.lock().await;
        if let Some(job) = jobs.get_mut(&params.id) {
            job.state = params.state;
            job.last_error = params.last_error.clone();
            if let Some(attempts) = params.attempts {
                job.attempts = attempts;
            }
            if let Some(run_at) = params.run_at {
                job.run_at = run_at;
            }
            if let Some(priority) = params.priority {
                job.priority = priority;
            }
            job.done_at = Some(OffsetDateTime::now_utc());
        }
        Ok(())
    }

    async fn find_job(&self, id: &str) -> Result<Option<JobRecord>, RepoError> {
        Ok(self.jobs.lock().await.get(id).cloned())
    }

    async fn list_jobs(
        &self,
        _filter: &JobQueryFilter,
        _page: PageRequest<JobCursor>,
    ) -> Result<CursorPage<JobRecord>, RepoError> {
        let jobs = self.jobs.lock().await;
        Ok(CursorPage {
            items: jobs.values().cloned().collect(),
            next_cursor: None,
        })
    }
}

async fn build_state(pool: PgPool) -> (ApiState, String) {
    let repos = Arc::new(PostgresRepositories::new(pool));

    let posts_repo: Arc<dyn PostsRepo> = repos.clone();
    let posts_write_repo: Arc<dyn PostsWriteRepo> = repos.clone();
    let sections_repo: Arc<dyn SectionsRepo> = repos.clone();
    let tags_repo: Arc<dyn TagsRepo> = repos.clone();
    let tags_write_repo: Arc<dyn TagsWriteRepo> = repos.clone();
    let settings_repo: Arc<dyn SettingsRepo> = repos.clone();
    let navigation_repo: Arc<dyn NavigationRepo> = repos.clone();
    let navigation_write_repo: Arc<dyn NavigationWriteRepo> = repos.clone();
    let pages_repo: Arc<dyn PagesRepo> = repos.clone();
    let pages_write_repo: Arc<dyn PagesWriteRepo> = repos.clone();
    let uploads_repo: Arc<dyn UploadsRepo> = repos.clone();
    let audit_repo: Arc<dyn AuditRepo> = repos.clone();
    let jobs_repo: Arc<dyn JobsRepo> = Arc::new(ImmediateJobsRepo::default());
    let api_keys_repo: Arc<dyn ApiKeysRepo> = repos.clone();

    let _feed_service = Arc::new(FeedService::new(
        posts_repo.clone(),
        sections_repo.clone(),
        tags_repo.clone(),
        settings_repo.clone(),
    ));
    let _page_service = Arc::new(PageService::new(pages_repo.clone()));
    let _chrome_service = Arc::new(ChromeService::new(
        navigation_repo.clone(),
        settings_repo.clone(),
    ));

    let audit_service = AdminAuditService::new(audit_repo.clone());
    let admin_post_service = Arc::new(AdminPostService::new(
        posts_repo.clone(),
        posts_write_repo.clone(),
        sections_repo.clone(),
        jobs_repo.clone(),
        tags_repo.clone(),
        audit_service.clone(),
    ));
    let admin_page_service = Arc::new(AdminPageService::new(
        pages_repo.clone(),
        pages_write_repo.clone(),
        jobs_repo.clone(),
        audit_service.clone(),
    ));
    let admin_tag_service = Arc::new(AdminTagService::new(
        tags_repo.clone(),
        tags_write_repo.clone(),
        audit_service.clone(),
    ));
    let admin_navigation_service = Arc::new(AdminNavigationService::new(
        navigation_repo.clone(),
        navigation_write_repo.clone(),
        pages_repo.clone(),
        audit_service.clone(),
    ));
    let admin_settings_service = Arc::new(AdminSettingsService::new(
        settings_repo.clone(),
        audit_service.clone(),
    ));
    let admin_upload_service = Arc::new(AdminUploadService::new(
        uploads_repo.clone(),
        audit_service.clone(),
    ));
    let admin_job_service = Arc::new(soffio::application::admin::jobs::AdminJobService::new(
        jobs_repo.clone(),
        audit_service.clone(),
    ));
    let admin_audit_service = Arc::new(audit_service);
    let api_key_service = Arc::new(ApiKeyService::new(api_keys_repo.clone()));

    let upload_storage =
        Arc::new(UploadStorage::new(std::path::PathBuf::from("uploads")).expect("upload storage"));
    let _response_cache = Arc::new(ResponseCache::new());

    let api_state = ApiState {
        api_keys: api_key_service.clone(),
        posts: admin_post_service,
        pages: admin_page_service,
        tags: admin_tag_service,
        navigation: admin_navigation_service,
        uploads: admin_upload_service,
        settings: admin_settings_service,
        jobs: admin_job_service,
        audit: admin_audit_service,
        db: repos.clone(),
        upload_storage,
        rate_limiter: Arc::new(soffio::infra::http::api::rate_limit::ApiRateLimiter::new(
            std::time::Duration::from_secs(60),
            200,
        )),
    };

    let issued = api_key_service
        .issue(IssueApiKeyCommand {
            name: "test".to_string(),
            description: None,
            scopes: vec![
                ApiScope::ContentRead,
                ApiScope::ContentWrite,
                ApiScope::TagWrite,
                ApiScope::NavigationWrite,
                ApiScope::UploadWrite,
                ApiScope::SettingsWrite,
                ApiScope::JobsRead,
            ],
            expires_at: None,
            created_by: "tests".to_string(),
        })
        .await
        .unwrap();

    (api_state, issued.token)
}

// ============ Posts ============

#[sqlx::test(migrations = "./migrations")]
async fn api_can_create_and_list_posts(pool: PgPool) {
    let (state, token) = build_state(pool).await;
    let principal = state.api_keys.authenticate(&token).await.unwrap();
    let post_payload = PostCreateRequest {
        title: "handler-post".into(),
        excerpt: "excerpt".into(),
        body_markdown: "# body".into(),
        summary_markdown: None,
        status: soffio::domain::types::PostStatus::Draft,
        pinned: false,
        scheduled_at: None,
        published_at: None,
        archived_at: None,
    };

    let _response = handlers::create_post(
        State(state.clone()),
        Extension(principal.clone()),
        Json(post_payload),
    )
    .await
    .expect("create post via handler");

    let _list = handlers::list_posts(
        State(state.clone()),
        Extension(principal.clone()),
        Query(handlers::PostListQuery {
            status: None,
            search: None,
            tag: None,
            month: None,
            cursor: None,
            limit: Some(10),
        }),
    )
    .await
    .expect("list posts via handler");
}

#[sqlx::test(migrations = "./migrations")]
async fn api_can_update_post_content(pool: PgPool) {
    let (state, token) = build_state(pool).await;
    let principal = state.api_keys.authenticate(&token).await.unwrap();

    // Create a post via service to get the ID
    let post = state
        .posts
        .create_post(
            "test",
            soffio::application::admin::posts::CreatePostCommand {
                title: "original-title".into(),
                excerpt: "original".into(),
                body_markdown: "# original".into(),
                summary_markdown: None,
                status: soffio::domain::types::PostStatus::Draft,
                pinned: false,
                scheduled_at: None,
                published_at: None,
                archived_at: None,
            },
        )
        .await
        .expect("create post via service");

    // Update the post via handler
    let update_payload = PostUpdateRequest {
        slug: post.slug.clone(),
        title: "updated-title".into(),
        excerpt: "updated".into(),
        body_markdown: "# updated".into(),
        summary_markdown: None,
        pinned: true,
    };

    let _updated = handlers::update_post(
        State(state.clone()),
        Extension(principal.clone()),
        axum::extract::Path(post.id),
        Json(update_payload),
    )
    .await
    .expect("update post via handler");
}

#[sqlx::test(migrations = "./migrations")]
async fn api_can_update_post_status(pool: PgPool) {
    let (state, token) = build_state(pool).await;
    let principal = state.api_keys.authenticate(&token).await.unwrap();

    // Create a post via service to get the ID
    let post = state
        .posts
        .create_post(
            "test",
            soffio::application::admin::posts::CreatePostCommand {
                title: "status-test".into(),
                excerpt: "excerpt".into(),
                body_markdown: "# body".into(),
                summary_markdown: None,
                status: soffio::domain::types::PostStatus::Draft,
                pinned: false,
                scheduled_at: None,
                published_at: None,
                archived_at: None,
            },
        )
        .await
        .expect("create post via service");

    // Update status to published via handler
    let status_payload = PostStatusRequest {
        status: soffio::domain::types::PostStatus::Published,
        scheduled_at: None,
        published_at: Some(OffsetDateTime::now_utc()),
        archived_at: None,
    };

    let _updated = handlers::update_post_status(
        State(state.clone()),
        Extension(principal.clone()),
        axum::extract::Path(post.id),
        Json(status_payload),
    )
    .await
    .expect("update post status via handler");
}

// ============ Pages ============

#[sqlx::test(migrations = "./migrations")]
async fn api_can_create_and_list_pages(pool: PgPool) {
    let (state, token) = build_state(pool).await;
    let principal = state.api_keys.authenticate(&token).await.unwrap();

    let page_payload = PageCreateRequest {
        slug: None,
        title: "test-page".into(),
        body_markdown: "# Page content".into(),
        status: soffio::domain::types::PageStatus::Draft,
        scheduled_at: None,
        published_at: None,
        archived_at: None,
    };

    let _response = handlers::create_page(
        State(state.clone()),
        Extension(principal.clone()),
        Json(page_payload),
    )
    .await
    .expect("create page via handler");

    let _list = handlers::list_pages(
        State(state.clone()),
        Extension(principal.clone()),
        Query(handlers::PageListQuery {
            status: None,
            search: None,
            month: None,
            cursor: None,
            limit: Some(10),
        }),
    )
    .await
    .expect("list pages via handler");
}

#[sqlx::test(migrations = "./migrations")]
async fn api_can_update_page_content(pool: PgPool) {
    let (state, token) = build_state(pool).await;
    let principal = state.api_keys.authenticate(&token).await.unwrap();

    // Create a page via service to get the ID
    let page = state
        .pages
        .create_page(
            "test",
            soffio::application::admin::pages::CreatePageCommand {
                title: "original-page".into(),
                body_markdown: "# original".into(),
                status: soffio::domain::types::PageStatus::Draft,
                scheduled_at: None,
                published_at: None,
                archived_at: None,
            },
        )
        .await
        .expect("create page via service");

    // Update the page via handler
    let update_payload = PageUpdateRequest {
        slug: page.slug.clone(),
        title: "updated-page".into(),
        body_markdown: "# updated".into(),
    };

    let _updated = handlers::update_page(
        State(state.clone()),
        Extension(principal.clone()),
        axum::extract::Path(page.id),
        Json(update_payload),
    )
    .await
    .expect("update page via handler");
}

#[sqlx::test(migrations = "./migrations")]
async fn api_can_update_page_status(pool: PgPool) {
    let (state, token) = build_state(pool).await;
    let principal = state.api_keys.authenticate(&token).await.unwrap();

    // Create a page via service to get the ID
    let page = state
        .pages
        .create_page(
            "test",
            soffio::application::admin::pages::CreatePageCommand {
                title: "status-page".into(),
                body_markdown: "# content".into(),
                status: soffio::domain::types::PageStatus::Draft,
                scheduled_at: None,
                published_at: None,
                archived_at: None,
            },
        )
        .await
        .expect("create page via service");

    // Update status via handler
    let status_payload = PageStatusRequest {
        status: soffio::domain::types::PageStatus::Published,
        scheduled_at: None,
        published_at: Some(OffsetDateTime::now_utc()),
        archived_at: None,
    };

    let _updated = handlers::update_page_status(
        State(state.clone()),
        Extension(principal.clone()),
        axum::extract::Path(page.id),
        Json(status_payload),
    )
    .await
    .expect("update page status via handler");
}

// ============ Tags ============

#[sqlx::test(migrations = "./migrations")]
async fn api_can_create_and_list_tags(pool: PgPool) {
    let (state, token) = build_state(pool).await;
    let principal = state.api_keys.authenticate(&token).await.unwrap();

    let tag_payload = TagCreateRequest {
        name: "test-tag".into(),
        description: Some("A test tag".into()),
        pinned: false,
    };

    let _response = handlers::create_tag(
        State(state.clone()),
        Extension(principal.clone()),
        Json(tag_payload),
    )
    .await
    .expect("create tag via handler");

    let _list = handlers::list_tags(
        State(state.clone()),
        Extension(principal.clone()),
        Query(handlers::TagListQuery {
            search: None,
            month: None,
            cursor: None,
            limit: Some(10),
            pinned: None,
        }),
    )
    .await
    .expect("list tags via handler");
}

#[sqlx::test(migrations = "./migrations")]
async fn api_can_update_tag(pool: PgPool) {
    let (state, token) = build_state(pool).await;
    let principal = state.api_keys.authenticate(&token).await.unwrap();

    // Create a tag via service to get the ID
    let tag = state
        .tags
        .create_tag(
            "test",
            soffio::application::admin::tags::CreateTagCommand {
                name: "original-tag".into(),
                description: None,
                pinned: false,
            },
        )
        .await
        .expect("create tag via service");

    // Update the tag via handler
    let update_payload = TagUpdateRequest {
        name: "updated-tag".into(),
        description: Some("Updated description".into()),
        pinned: true,
    };

    let _updated = handlers::update_tag(
        State(state.clone()),
        Extension(principal.clone()),
        axum::extract::Path(tag.id),
        Json(update_payload),
    )
    .await
    .expect("update tag via handler");
}

#[sqlx::test(migrations = "./migrations")]
async fn api_can_delete_tag(pool: PgPool) {
    let (state, token) = build_state(pool).await;
    let principal = state.api_keys.authenticate(&token).await.unwrap();

    // Create a tag via service to get the ID
    let tag = state
        .tags
        .create_tag(
            "test",
            soffio::application::admin::tags::CreateTagCommand {
                name: "deletable-tag".into(),
                description: None,
                pinned: false,
            },
        )
        .await
        .expect("create tag via service");

    // Delete the tag via handler
    let _deleted = handlers::delete_tag(
        State(state.clone()),
        Extension(principal.clone()),
        axum::extract::Path(tag.id),
    )
    .await
    .expect("delete tag via handler");
}

// ============ Navigation ============

#[sqlx::test(migrations = "./migrations")]
async fn api_can_create_and_list_navigation(pool: PgPool) {
    let (state, token) = build_state(pool).await;
    let principal = state.api_keys.authenticate(&token).await.unwrap();

    let nav_payload = NavigationCreateRequest {
        label: "Home".into(),
        destination_type: soffio::domain::types::NavigationDestinationType::External,
        destination_page_id: None,
        destination_url: Some("https://example.com".into()),
        sort_order: 1,
        visible: true,
        open_in_new_tab: false,
    };

    let _response = handlers::create_navigation(
        State(state.clone()),
        Extension(principal.clone()),
        Json(nav_payload),
    )
    .await
    .expect("create navigation via handler");

    let _list = handlers::list_navigation(
        State(state.clone()),
        Extension(principal.clone()),
        Query(handlers::NavigationListQuery {
            search: None,
            cursor: None,
            limit: Some(10),
            visible: None,
        }),
    )
    .await
    .expect("list navigation via handler");
}

#[sqlx::test(migrations = "./migrations")]
async fn api_can_update_navigation(pool: PgPool) {
    let (state, token) = build_state(pool).await;
    let principal = state.api_keys.authenticate(&token).await.unwrap();

    // Create navigation item via service to get the ID
    let nav = state
        .navigation
        .create_item(
            "test",
            soffio::application::admin::navigation::CreateNavigationItemCommand {
                label: "Original".into(),
                destination_type: soffio::domain::types::NavigationDestinationType::External,
                destination_page_id: None,
                destination_url: Some("https://original.com".into()),
                sort_order: 1,
                visible: true,
                open_in_new_tab: false,
            },
        )
        .await
        .expect("create navigation via service");

    // Update navigation via handler
    let update_payload = NavigationUpdateRequest {
        label: "Updated".into(),
        destination_type: soffio::domain::types::NavigationDestinationType::External,
        destination_page_id: None,
        destination_url: Some("https://updated.com".into()),
        sort_order: 2,
        visible: false,
        open_in_new_tab: true,
    };

    let _updated = handlers::update_navigation(
        State(state.clone()),
        Extension(principal.clone()),
        axum::extract::Path(nav.id),
        Json(update_payload),
    )
    .await
    .expect("update navigation via handler");
}

#[sqlx::test(migrations = "./migrations")]
async fn api_can_delete_navigation(pool: PgPool) {
    let (state, token) = build_state(pool).await;
    let principal = state.api_keys.authenticate(&token).await.unwrap();

    // Create navigation item via service to get the ID
    let nav = state
        .navigation
        .create_item(
            "test",
            soffio::application::admin::navigation::CreateNavigationItemCommand {
                label: "Deletable".into(),
                destination_type: soffio::domain::types::NavigationDestinationType::External,
                destination_page_id: None,
                destination_url: Some("https://delete.me".into()),
                sort_order: 99,
                visible: true,
                open_in_new_tab: false,
            },
        )
        .await
        .expect("create navigation via service");

    // Delete navigation via handler
    let _deleted = handlers::delete_navigation(
        State(state.clone()),
        Extension(principal.clone()),
        axum::extract::Path(nav.id),
    )
    .await
    .expect("delete navigation via handler");
}

// ============ Uploads ============

#[sqlx::test(migrations = "./migrations")]
async fn api_can_list_uploads(pool: PgPool) {
    let (state, token) = build_state(pool).await;
    let principal = state.api_keys.authenticate(&token).await.unwrap();

    let _list = handlers::list_uploads(
        State(state.clone()),
        Extension(principal.clone()),
        Query(handlers::UploadListQuery {
            search: None,
            content_type: None,
            month: None,
            cursor: None,
            limit: Some(10),
        }),
    )
    .await
    .expect("list uploads via handler");
}

// ============ Settings ============

#[sqlx::test(migrations = "./migrations")]
async fn api_can_get_and_patch_settings(pool: PgPool) {
    let (state, token) = build_state(pool).await;
    let principal = state.api_keys.authenticate(&token).await.unwrap();

    // Get settings
    let _settings = handlers::get_settings(State(state.clone()), Extension(principal.clone()))
        .await
        .expect("get settings via handler");

    // Patch settings
    let patch_payload = SettingsPatchRequest {
        brand_title: Some("Updated Title".into()),
        brand_href: None,
        footer_copy: None,
        homepage_size: Some(15),
        admin_page_size: None,
        show_tag_aggregations: None,
        show_month_aggregations: None,
        tag_filter_limit: None,
        month_filter_limit: None,
        timezone: None,
        meta_title: None,
        meta_description: None,
        og_title: None,
        og_description: None,
        public_site_url: None,
    };

    let _patched = handlers::patch_settings(
        State(state.clone()),
        Extension(principal.clone()),
        Json(patch_payload),
    )
    .await
    .expect("patch settings via handler");
}

// ============ Jobs ============

#[sqlx::test(migrations = "./migrations")]
async fn api_can_list_jobs(pool: PgPool) {
    let (state, token) = build_state(pool).await;
    let principal = state.api_keys.authenticate(&token).await.unwrap();

    let _list = handlers::list_jobs(
        State(state.clone()),
        Extension(principal.clone()),
        Query(handlers::JobsListQuery {
            state: None,
            job_type: None,
            search: None,
            cursor: None,
            limit: Some(10),
        }),
    )
    .await
    .expect("list jobs via handler");
}

// ============ Audit ============

#[sqlx::test(migrations = "./migrations")]
async fn api_can_list_audit_logs(pool: PgPool) {
    let (state, _token) = build_state(pool).await;

    // Add AuditRead scope for this test
    let issued = state
        .api_keys
        .issue(IssueApiKeyCommand {
            name: "audit-test".to_string(),
            description: None,
            scopes: vec![ApiScope::AuditRead],
            expires_at: None,
            created_by: "tests".to_string(),
        })
        .await
        .unwrap();

    let principal = state.api_keys.authenticate(&issued.token).await.unwrap();

    let _list = handlers::list_audit_logs(
        State(state.clone()),
        Extension(principal.clone()),
        Query(handlers::AuditListQuery {
            actor: None,
            action: None,
            entity_type: None,
            search: None,
            cursor: None,
            limit: Some(10),
        }),
    )
    .await
    .expect("list audit logs via handler");
}
