use std::sync::Arc;

use crate::application::admin::audit::AdminAuditService;
use crate::application::admin::jobs::AdminJobService;
use crate::application::admin::navigation::AdminNavigationService;
use crate::application::admin::pages::AdminPageService;
use crate::application::admin::posts::AdminPostService;
use crate::application::admin::settings::AdminSettingsService;
use crate::application::admin::snapshots::AdminSnapshotService;
use crate::application::admin::tags::AdminTagService;
use crate::application::admin::uploads::AdminUploadService;
use crate::application::api_keys::{ApiKeyService, ApiPrincipal};
use crate::infra::db::PostgresRepositories;
use crate::infra::uploads::UploadStorage;

use super::rate_limit::ApiRateLimiter;

#[derive(Clone)]
pub struct ApiState {
    pub api_keys: Arc<ApiKeyService>,
    pub posts: Arc<AdminPostService>,
    pub pages: Arc<AdminPageService>,
    pub tags: Arc<AdminTagService>,
    pub navigation: Arc<AdminNavigationService>,
    pub uploads: Arc<AdminUploadService>,
    pub settings: Arc<AdminSettingsService>,
    pub jobs: Arc<AdminJobService>,
    pub audit: Arc<AdminAuditService>,
    pub snapshots: Arc<AdminSnapshotService>,
    pub db: Arc<PostgresRepositories>,
    pub upload_storage: Arc<UploadStorage>,
    pub rate_limiter: Arc<ApiRateLimiter>,
}

impl ApiState {
    pub fn actor_label(principal: &ApiPrincipal) -> String {
        format!("api_key:{}:{}", principal.prefix, principal.name)
    }
}
