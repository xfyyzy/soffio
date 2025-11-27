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

#[sqlx::test(migrations = "./migrations")]
async fn api_can_create_and_list_post_via_handlers(pool: PgPool) {
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
