use axum::body::to_bytes;
use axum::extract::{Extension, Json, Path, Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
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
use soffio::application::admin::snapshots::AdminSnapshotService;
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
use soffio::domain::entities::{JobRecord, UploadRecord};
use soffio::domain::types::JobState;
use soffio::infra::cache::{CacheWarmDebouncer, ResponseCache};
use soffio::infra::db::PostgresRepositories;
use soffio::infra::http::api::handlers;
use soffio::infra::http::api::models::*;
use soffio::infra::http::api::state::ApiState;
use soffio::infra::uploads::UploadStorage;
use uuid::Uuid;

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

    async fn count_jobs(&self, filter: &JobQueryFilter) -> Result<u64, RepoError> {
        let jobs = self.jobs.lock().await;
        let count = jobs
            .values()
            .filter(|job| {
                if let Some(state) = filter.state
                    && job.state != state
                {
                    return false;
                }
                if let Some(job_type) = filter.job_type
                    && job.job_type != job_type
                {
                    return false;
                }
                if let Some(search) = &filter.search {
                    let search_lower = search.to_lowercase();
                    let payload_str = job.payload.to_string().to_lowercase();
                    let last_error_str = job
                        .last_error
                        .as_ref()
                        .map(|e| e.to_lowercase())
                        .unwrap_or_default();
                    if !payload_str.contains(&search_lower)
                        && !last_error_str.contains(&search_lower)
                    {
                        return false;
                    }
                }
                true
            })
            .count();
        Ok(count as u64)
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
        settings_repo.clone(),
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
        snapshots: Arc::new(AdminSnapshotService::new(repos.clone())),
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
                ApiScope::PostRead,
                ApiScope::PostWrite,
                ApiScope::PageRead,
                ApiScope::PageWrite,
                ApiScope::TagRead,
                ApiScope::TagWrite,
                ApiScope::NavigationRead,
                ApiScope::NavigationWrite,
                ApiScope::UploadRead,
                ApiScope::UploadWrite,
                ApiScope::SettingsRead,
                ApiScope::SettingsWrite,
                ApiScope::JobRead,
                ApiScope::AuditRead,
                ApiScope::SnapshotRead,
                ApiScope::SnapshotWrite,
            ],
            expires_in: None,
            created_by: "tests".to_string(),
        })
        .await
        .unwrap();

    (api_state, issued.token)
}

async fn response_json(resp: impl IntoResponse) -> (StatusCode, serde_json::Value) {
    let response = resp.into_response();
    let status = response.status();
    let body = to_bytes(response.into_body(), 1_048_576)
        .await
        .expect("read body");
    let value = serde_json::from_slice(&body).expect("decode json");
    (status, value)
}

fn string_field<'a>(value: &'a serde_json::Value, key: &str) -> &'a str {
    value
        .get(key)
        .and_then(serde_json::Value::as_str)
        .unwrap_or("")
}

fn uuid_field(value: &serde_json::Value, key: &str) -> Uuid {
    Uuid::parse_str(string_field(value, key)).expect("uuid field")
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

    let (status, created_post) = response_json(
        handlers::create_post(
            State(state.clone()),
            Extension(principal.clone()),
            Json(post_payload),
        )
        .await
        .expect("create post via handler"),
    )
    .await;

    assert_eq!(status, StatusCode::CREATED);
    let created_post_id = string_field(&created_post, "id").to_string();
    let created_post_slug = string_field(&created_post, "slug").to_string();

    let (status, found_by_id) = response_json(
        handlers::get_post_by_id(
            State(state.clone()),
            Extension(principal.clone()),
            Path(created_post_id.parse().unwrap()),
        )
        .await
        .expect("get post by id"),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(string_field(&found_by_id, "id"), created_post_id.as_str());

    let (status, found_by_slug) = response_json(
        handlers::get_post(
            State(state.clone()),
            Extension(principal.clone()),
            Path(created_post_slug.clone()),
        )
        .await
        .expect("get post by slug"),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        string_field(&found_by_slug, "slug"),
        created_post_slug.as_str()
    );

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

#[sqlx::test(migrations = "./migrations")]
async fn api_can_partial_update_post(pool: PgPool) {
    let (state, token) = build_state(pool).await;
    let principal = state.api_keys.authenticate(&token).await.unwrap();

    let post = state
        .posts
        .create_post(
            "test",
            soffio::application::admin::posts::CreatePostCommand {
                title: "partial".into(),
                excerpt: "orig".into(),
                body_markdown: "# body".into(),
                summary_markdown: Some("sum".into()),
                status: soffio::domain::types::PostStatus::Draft,
                pinned: false,
                scheduled_at: None,
                published_at: None,
                archived_at: None,
            },
        )
        .await
        .expect("create post via service");

    handlers::update_post_pin(
        State(state.clone()),
        Extension(principal.clone()),
        axum::extract::Path(post.id),
        Json(PostPinRequest { pinned: true }),
    )
    .await
    .expect("pin post");
    let mut latest = state.posts.load_post(post.id).await.unwrap().unwrap();
    assert!(latest.pinned);

    handlers::update_post_title(
        State(state.clone()),
        Extension(principal.clone()),
        axum::extract::Path(post.id),
        Json(PostTitleRequest {
            title: "new title".into(),
        }),
    )
    .await
    .expect("update title");
    latest = state.posts.load_post(post.id).await.unwrap().unwrap();
    assert_eq!(latest.title, "new title");

    handlers::update_post_excerpt(
        State(state.clone()),
        Extension(principal.clone()),
        axum::extract::Path(post.id),
        Json(PostExcerptRequest {
            excerpt: "new excerpt".into(),
        }),
    )
    .await
    .expect("update excerpt");
    latest = state.posts.load_post(post.id).await.unwrap().unwrap();
    assert_eq!(latest.excerpt, "new excerpt");

    handlers::update_post_body(
        State(state.clone()),
        Extension(principal.clone()),
        axum::extract::Path(post.id),
        Json(PostBodyRequest {
            body_markdown: "## changed".into(),
        }),
    )
    .await
    .expect("update body");
    latest = state.posts.load_post(post.id).await.unwrap().unwrap();
    assert_eq!(latest.body_markdown, "## changed");

    handlers::update_post_summary(
        State(state.clone()),
        Extension(principal),
        axum::extract::Path(post.id),
        Json(PostSummaryRequest {
            summary_markdown: Some("updated summary".into()),
        }),
    )
    .await
    .expect("update summary");
    latest = state.posts.load_post(post.id).await.unwrap().unwrap();
    assert_eq!(latest.summary_markdown.as_deref(), Some("updated summary"));
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

    let (status, created_page) = response_json(
        handlers::create_page(
            State(state.clone()),
            Extension(principal.clone()),
            Json(page_payload),
        )
        .await
        .expect("create page via handler"),
    )
    .await;

    assert_eq!(status, StatusCode::CREATED);
    let created_page_id = string_field(&created_page, "id").to_string();
    let created_page_slug = string_field(&created_page, "slug").to_string();

    let (status, found_by_id) = response_json(
        handlers::get_page_by_id(
            State(state.clone()),
            Extension(principal.clone()),
            Path(created_page_id.parse().unwrap()),
        )
        .await
        .expect("get page by id"),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(string_field(&found_by_id, "id"), created_page_id.as_str());

    let (status, found_by_slug) = response_json(
        handlers::get_page(
            State(state.clone()),
            Extension(principal.clone()),
            Path(created_page_slug.clone()),
        )
        .await
        .expect("get page by slug"),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        string_field(&found_by_slug, "slug"),
        created_page_slug.as_str()
    );

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

#[sqlx::test(migrations = "./migrations")]
async fn api_can_partial_update_page(pool: PgPool) {
    let (state, token) = build_state(pool).await;
    let principal = state.api_keys.authenticate(&token).await.unwrap();

    let page = state
        .pages
        .create_page(
            "test",
            soffio::application::admin::pages::CreatePageCommand {
                title: "page".into(),
                body_markdown: "hello".into(),
                status: soffio::domain::types::PageStatus::Draft,
                scheduled_at: None,
                published_at: None,
                archived_at: None,
            },
        )
        .await
        .expect("create page");

    handlers::update_page_title(
        State(state.clone()),
        Extension(principal.clone()),
        axum::extract::Path(page.id),
        Json(PageTitleRequest {
            title: "new page".into(),
        }),
    )
    .await
    .expect("update page title");

    let mut latest = state.pages.find_by_id(page.id).await.unwrap().unwrap();
    assert_eq!(latest.title, "new page");

    handlers::update_page_body(
        State(state.clone()),
        Extension(principal),
        axum::extract::Path(page.id),
        Json(PageBodyRequest {
            body_markdown: "updated body".into(),
        }),
    )
    .await
    .expect("update page body");

    latest = state.pages.find_by_id(page.id).await.unwrap().unwrap();
    assert_eq!(latest.body_markdown, "updated body");
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

    let (status, created_tag) = response_json(
        handlers::create_tag(
            State(state.clone()),
            Extension(principal.clone()),
            Json(tag_payload),
        )
        .await
        .expect("create tag via handler"),
    )
    .await;

    assert_eq!(status, StatusCode::CREATED);
    let created_tag_id = string_field(&created_tag, "id").to_string();
    let created_tag_slug = string_field(&created_tag, "slug").to_string();

    let (status, found_by_id) = response_json(
        handlers::get_tag_by_id(
            State(state.clone()),
            Extension(principal.clone()),
            Path(created_tag_id.parse().unwrap()),
        )
        .await
        .expect("get tag by id"),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(string_field(&found_by_id, "id"), created_tag_id.as_str());

    let (status, found_by_slug) = response_json(
        handlers::get_tag_by_slug(
            State(state.clone()),
            Extension(principal.clone()),
            Path(created_tag_slug.clone()),
        )
        .await
        .expect("get tag by slug"),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        string_field(&found_by_slug, "slug"),
        created_tag_slug.as_str()
    );

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

#[sqlx::test(migrations = "./migrations")]
async fn api_can_partial_update_tag(pool: PgPool) {
    let (state, token) = build_state(pool).await;
    let principal = state.api_keys.authenticate(&token).await.unwrap();

    let tag = state
        .tags
        .create_tag(
            "test",
            soffio::application::admin::tags::CreateTagCommand {
                name: "tag".into(),
                description: Some("desc".into()),
                pinned: false,
            },
        )
        .await
        .expect("create tag");

    handlers::update_tag_pin(
        State(state.clone()),
        Extension(principal.clone()),
        axum::extract::Path(tag.id),
        Json(TagPinRequest { pinned: true }),
    )
    .await
    .expect("pin tag");

    handlers::update_tag_name(
        State(state.clone()),
        Extension(principal.clone()),
        axum::extract::Path(tag.id),
        Json(TagNameRequest {
            name: "renamed".into(),
        }),
    )
    .await
    .expect("rename tag");

    handlers::update_tag_description(
        State(state.clone()),
        Extension(principal),
        axum::extract::Path(tag.id),
        Json(TagDescriptionRequest {
            description: Some("new description".into()),
        }),
    )
    .await
    .expect("update tag description");

    let latest = state.tags.find_by_id(tag.id).await.unwrap().unwrap();
    assert!(latest.pinned);
    assert_eq!(latest.name, "renamed");
    assert_eq!(latest.description.as_deref(), Some("new description"));
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

    let (status, created_nav) = response_json(
        handlers::create_navigation(
            State(state.clone()),
            Extension(principal.clone()),
            Json(nav_payload),
        )
        .await
        .expect("create navigation via handler"),
    )
    .await;

    assert_eq!(status, StatusCode::CREATED);
    let created_id = string_field(&created_nav, "id").to_string();

    let (status, fetched) = response_json(
        handlers::get_navigation_item(
            State(state.clone()),
            Extension(principal.clone()),
            Path(created_id.parse().unwrap()),
        )
        .await
        .expect("get navigation by id"),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(string_field(&fetched, "id"), created_id.as_str());

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

#[sqlx::test(migrations = "./migrations")]
async fn api_can_partial_update_navigation(pool: PgPool) {
    let (state, token) = build_state(pool).await;
    let principal = state.api_keys.authenticate(&token).await.unwrap();

    let nav = state
        .navigation
        .create_item(
            "test",
            soffio::application::admin::navigation::CreateNavigationItemCommand {
                label: "Nav".into(),
                destination_type: soffio::domain::types::NavigationDestinationType::External,
                destination_page_id: None,
                destination_url: Some("https://example.com".into()),
                sort_order: 1,
                visible: true,
                open_in_new_tab: false,
            },
        )
        .await
        .expect("create navigation");

    handlers::update_navigation_label(
        State(state.clone()),
        Extension(principal.clone()),
        axum::extract::Path(nav.id),
        Json(NavigationLabelRequest {
            label: "Nav Updated".into(),
        }),
    )
    .await
    .expect("update label");

    handlers::update_navigation_destination(
        State(state.clone()),
        Extension(principal.clone()),
        axum::extract::Path(nav.id),
        Json(NavigationDestinationRequest {
            destination_type: soffio::domain::types::NavigationDestinationType::External,
            destination_page_id: None,
            destination_url: Some("https://example.org".into()),
        }),
    )
    .await
    .expect("update destination");

    handlers::update_navigation_sort_order(
        State(state.clone()),
        Extension(principal.clone()),
        axum::extract::Path(nav.id),
        Json(NavigationSortOrderRequest { sort_order: 5 }),
    )
    .await
    .expect("update sort order");

    handlers::update_navigation_visibility(
        State(state.clone()),
        Extension(principal.clone()),
        axum::extract::Path(nav.id),
        Json(NavigationVisibilityRequest { visible: false }),
    )
    .await
    .expect("update visibility");

    handlers::update_navigation_open_in_new_tab(
        State(state.clone()),
        Extension(principal),
        axum::extract::Path(nav.id),
        Json(NavigationOpenInNewTabRequest {
            open_in_new_tab: true,
        }),
    )
    .await
    .expect("update open in new tab");

    let latest = state.navigation.find_by_id(nav.id).await.unwrap().unwrap();
    assert_eq!(latest.label, "Nav Updated");
    assert_eq!(
        latest.destination_url.as_deref(),
        Some("https://example.org")
    );
    assert_eq!(latest.sort_order, 5);
    assert!(!latest.visible);
    assert!(latest.open_in_new_tab);
}

// ============ Uploads ============

#[sqlx::test(migrations = "./migrations")]
async fn api_can_list_uploads(pool: PgPool) {
    let (state, token) = build_state(pool).await;
    let principal = state.api_keys.authenticate(&token).await.unwrap();

    let upload = UploadRecord {
        id: Uuid::new_v4(),
        filename: "demo.txt".into(),
        content_type: "text/plain".into(),
        size_bytes: 4,
        checksum: "abcd".into(),
        stored_path: "uploads/demo.txt".into(),
        metadata: soffio::domain::uploads::UploadMetadata::default(),
        created_at: OffsetDateTime::now_utc(),
    };
    state
        .uploads
        .register_upload("tests", upload.clone())
        .await
        .expect("register upload");

    let (status, fetched) = response_json(
        handlers::get_upload(
            State(state.clone()),
            Extension(principal.clone()),
            Path(upload.id),
        )
        .await
        .expect("get upload by id"),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(string_field(&fetched, "id"), upload.id.to_string());

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
        global_toc_enabled: None,
        favicon_svg: None,
    };

    let _patched = handlers::patch_settings(
        State(state.clone()),
        Extension(principal.clone()),
        Json(patch_payload),
    )
    .await
    .expect("patch settings via handler");
}

#[sqlx::test(migrations = "./migrations")]
async fn api_settings_patch_includes_toc_and_favicon(pool: PgPool) {
    let (state, token) = build_state(pool).await;
    let principal = state.api_keys.authenticate(&token).await.unwrap();

    let patch_payload = SettingsPatchRequest {
        brand_title: None,
        brand_href: None,
        footer_copy: None,
        homepage_size: None,
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
        global_toc_enabled: Some(true),
        favicon_svg: Some("<svg></svg>".into()),
    };

    handlers::patch_settings(
        State(state.clone()),
        Extension(principal),
        Json(patch_payload),
    )
    .await
    .expect("patch settings toc/favicon");

    // Reload from repo to assert persisted values
    let latest = state.settings.load().await.unwrap();
    assert!(latest.global_toc_enabled);
    assert_eq!(latest.favicon_svg, "<svg></svg>");
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
            expires_in: None,
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

// ============ API Keys ============

#[sqlx::test(migrations = "./migrations")]
async fn api_can_get_api_key_info(pool: PgPool) {
    let (state, token) = build_state(pool).await;
    let principal = state.api_keys.authenticate(&token).await.unwrap();

    let Json(info) = handlers::get_api_key_info(State(state.clone()), Extension(principal))
        .await
        .expect("get api key info");

    assert_eq!(info.prefix.len(), 12);
    assert!(info.scopes.contains(&ApiScope::PostRead));
    assert_eq!(info.status, soffio::domain::api_keys::ApiKeyStatus::Active);
}

// ============ API Key Scope Granularity ============

#[sqlx::test(migrations = "./migrations")]
async fn api_scope_granularity_post_vs_page(pool: PgPool) {
    let (state, _token) = build_state(pool).await;

    // Issue a key with only PostRead scope
    let issued = state
        .api_keys
        .issue(IssueApiKeyCommand {
            name: "post-only".to_string(),
            description: None,
            scopes: vec![ApiScope::PostRead],
            expires_in: None,
            created_by: "tests".to_string(),
        })
        .await
        .unwrap();

    let principal = state.api_keys.authenticate(&issued.token).await.unwrap();

    // Should be able to list posts
    let _posts = handlers::list_posts(
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
    .expect("should be able to list posts with PostRead scope");

    // Should NOT be able to list pages (requires PageRead)
    assert!(
        principal.requires(ApiScope::PageRead).is_err(),
        "PostRead scope should not grant PageRead access"
    );
}

// ============ API Key Authentication Status ============

#[sqlx::test(migrations = "./migrations")]
async fn api_auth_rejects_revoked_key(pool: PgPool) {
    let (state, _token) = build_state(pool).await;

    // Issue a key
    let issued = state
        .api_keys
        .issue(IssueApiKeyCommand {
            name: "revoke-test".to_string(),
            description: None,
            scopes: vec![ApiScope::PostRead],
            expires_in: None,
            created_by: "tests".to_string(),
        })
        .await
        .unwrap();

    // Revoke the key
    state
        .api_keys
        .revoke(issued.record.id)
        .await
        .expect("revoke should succeed");

    // Authentication should fail with Revoked error
    let result = state.api_keys.authenticate(&issued.token).await;
    assert!(
        result.is_err(),
        "authentication should fail for revoked key"
    );
    let err = result.unwrap_err();
    assert!(
        matches!(err, soffio::application::api_keys::ApiAuthError::Revoked),
        "should get Revoked error, got: {:?}",
        err
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn api_auth_rejects_expired_key(pool: PgPool) {
    let (state, _token) = build_state(pool).await;

    // Issue a key that expires immediately (expires_in = 0 means expires_at = now)
    let issued = state
        .api_keys
        .issue(IssueApiKeyCommand {
            name: "expired-test".to_string(),
            description: None,
            scopes: vec![ApiScope::PostRead],
            expires_in: Some(time::Duration::ZERO),
            created_by: "tests".to_string(),
        })
        .await
        .unwrap();

    // Small delay to ensure we're past the expires_at time
    tokio::time::sleep(std::time::Duration::from_millis(10)).await;

    // Authentication should fail with Expired error
    let result = state.api_keys.authenticate(&issued.token).await;
    assert!(
        result.is_err(),
        "authentication should fail for expired key"
    );
    let err = result.unwrap_err();
    assert!(
        matches!(err, soffio::application::api_keys::ApiAuthError::Expired),
        "should get Expired error, got: {:?}",
        err
    );
}

// ============ API Key Rotation ============

#[sqlx::test(migrations = "./migrations")]
async fn api_rotate_reactivates_revoked_key(pool: PgPool) {
    let (state, _token) = build_state(pool).await;

    // Issue a key
    let issued = state
        .api_keys
        .issue(IssueApiKeyCommand {
            name: "rotate-revoke-test".to_string(),
            description: None,
            scopes: vec![ApiScope::PostRead],
            expires_in: None,
            created_by: "tests".to_string(),
        })
        .await
        .unwrap();

    // Revoke the key
    state
        .api_keys
        .revoke(issued.record.id)
        .await
        .expect("revoke should succeed");

    // Rotation should succeed and reactivate the key
    let rotated = state
        .api_keys
        .rotate(issued.record.id)
        .await
        .expect("rotation should succeed for revoked key");

    // The key should now be active
    assert_eq!(
        rotated.record.status,
        soffio::domain::api_keys::ApiKeyStatus::Active,
        "key should be reactivated after rotation"
    );

    // The new token should work for authentication
    let auth_result = state.api_keys.authenticate(&rotated.token).await;
    assert!(
        auth_result.is_ok(),
        "authentication should succeed with rotated token"
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn api_rotate_recalculates_expiration_preserves_created_at(pool: PgPool) {
    let (state, _token) = build_state(pool).await;

    // Issue a key with 30-day expiration duration
    let issued = state
        .api_keys
        .issue(IssueApiKeyCommand {
            name: "rotate-preserve-test".to_string(),
            description: None,
            scopes: vec![ApiScope::PostRead],
            expires_in: Some(time::Duration::days(30)),
            created_by: "tests".to_string(),
        })
        .await
        .unwrap();

    let original_created_at = issued.record.created_at;
    let original_expires_in = issued.record.expires_in;
    let original_expires_at = issued.record.expires_at;

    // Small delay to ensure recalculated expires_at is different
    tokio::time::sleep(std::time::Duration::from_millis(10)).await;

    // Rotate the key
    let rotated = state
        .api_keys
        .rotate(issued.record.id)
        .await
        .expect("rotation should succeed");

    // created_at and expires_in should be preserved
    assert_eq!(
        rotated.record.created_at, original_created_at,
        "created_at should be preserved after rotation"
    );
    assert_eq!(
        rotated.record.expires_in, original_expires_in,
        "expires_in duration should be preserved after rotation"
    );

    // expires_at should be recalculated (should be later than original)
    assert!(
        rotated.record.expires_at > original_expires_at,
        "expires_at should be recalculated to a later time after rotation"
    );

    // The token should be different
    assert_ne!(
        issued.token, rotated.token,
        "token should change after rotation"
    );

    // Old token should no longer work
    let old_auth = state.api_keys.authenticate(&issued.token).await;
    assert!(old_auth.is_err(), "old token should not authenticate");

    // New token should work
    let new_auth = state.api_keys.authenticate(&rotated.token).await;
    assert!(new_auth.is_ok(), "new token should authenticate");
}

#[sqlx::test(migrations = "./migrations")]
async fn api_snapshots_cover_create_get_list_and_rollback(pool: PgPool) {
    let (state, token) = build_state(pool).await;
    let principal = state.api_keys.authenticate(&token).await.unwrap();

    // Create post
    let (status, post_json) = response_json(
        handlers::create_post(
            State(state.clone()),
            Extension(principal.clone()),
            Json(PostCreateRequest {
                title: "snap-post".into(),
                excerpt: "excerpt".into(),
                body_markdown: "# body".into(),
                summary_markdown: None,
                status: soffio::domain::types::PostStatus::Draft,
                pinned: false,
                scheduled_at: None,
                published_at: None,
                archived_at: None,
            }),
        )
        .await
        .expect("create post"),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    let post_id = uuid_field(&post_json, "id");
    let post_slug = string_field(&post_json, "slug").to_string();

    // Tag to ensure tag list restored
    let (_, tag_json) = response_json(
        handlers::create_tag(
            State(state.clone()),
            Extension(principal.clone()),
            Json(TagCreateRequest {
                name: "snap-tag".into(),
                description: None,
                pinned: false,
            }),
        )
        .await
        .expect("create tag"),
    )
    .await;
    let tag_id = uuid_field(&tag_json, "id");
    let status = handlers::replace_post_tags(
        State(state.clone()),
        Extension(principal.clone()),
        Path(post_id),
        Json(PostTagsRequest {
            tag_ids: vec![tag_id],
        }),
    )
    .await
    .expect("attach tag")
    .into_response()
    .status();
    assert_eq!(status, StatusCode::NO_CONTENT);

    // Snapshot (version 1)
    let (status, snap_json) = response_json(
        handlers::create_snapshot(
            State(state.clone()),
            Extension(principal.clone()),
            Json(SnapshotCreateRequest {
                entity_type: soffio::domain::types::SnapshotEntityType::Post,
                entity_id: post_id,
                description: Some("v1".into()),
            }),
        )
        .await
        .expect("create snapshot"),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    let snapshot_id = uuid_field(&snap_json, "id");

    // Mutate post & tags
    let (status, _) = response_json(
        handlers::update_post(
            State(state.clone()),
            Extension(principal.clone()),
            Path(post_id),
            Json(PostUpdateRequest {
                slug: post_slug.clone(),
                title: "changed".into(),
                excerpt: "changed excerpt".into(),
                body_markdown: "changed body".into(),
                summary_markdown: None,
                pinned: false,
            }),
        )
        .await
        .expect("update post"),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let status = handlers::replace_post_tags(
        State(state.clone()),
        Extension(principal.clone()),
        Path(post_id),
        Json(PostTagsRequest { tag_ids: vec![] }),
    )
    .await
    .expect("clear tags")
    .into_response()
    .status();
    assert_eq!(status, StatusCode::NO_CONTENT);

    // Rollback
    let (status, _) = response_json(
        handlers::rollback_snapshot(
            State(state.clone()),
            Extension(principal.clone()),
            Path(snapshot_id),
        )
        .await
        .expect("rollback snapshot"),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    // Verify post restored
    let restored = state
        .posts
        .load_post(post_id)
        .await
        .expect("find post")
        .expect("post exists");
    assert_eq!(restored.title, "snap-post");
    assert_eq!(restored.excerpt, "excerpt");

    let restored_tags: Vec<Uuid> = sqlx::query_scalar::<_, Uuid>(
        "SELECT tag_id FROM post_tags WHERE post_id = $1 ORDER BY tag_id",
    )
    .bind(post_id)
    .fetch_all(state.db.pool())
    .await
    .expect("post tags");
    assert_eq!(restored_tags, vec![tag_id]);

    // List & get
    let (status, list_json) = response_json(
        handlers::list_snapshots(
            State(state.clone()),
            Query(SnapshotListQuery {
                entity_type: Some(soffio::domain::types::SnapshotEntityType::Post),
                entity_id: Some(post_id),
                search: None,
                cursor: None,
                limit: Some(10),
            }),
            Extension(principal.clone()),
        )
        .await
        .expect("list snapshots"),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        list_json
            .get("items")
            .and_then(|v| v.as_array())
            .map(Vec::len)
            .unwrap_or(0),
        1
    );

    let (status, get_json) = response_json(
        handlers::get_snapshot(
            State(state.clone()),
            Path(snapshot_id),
            Extension(principal),
        )
        .await
        .expect("get snapshot"),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(uuid_field(&get_json, "id"), snapshot_id);
}
