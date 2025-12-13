use std::sync::Arc;

use crate::application::admin::{
    audit::AdminAuditService, chrome::AdminChromeService, dashboard::AdminDashboardService,
    jobs::AdminJobService, navigation::AdminNavigationService, pages::AdminPageService,
    posts::AdminPostService, settings::AdminSettingsService, snapshots::AdminSnapshotService,
    tags::AdminTagService, uploads::AdminUploadService,
};
use crate::application::api_keys::ApiKeyService;
use crate::infra::{
    cache::{CacheWarmDebouncer, ResponseCache},
    db::PostgresRepositories,
    uploads::UploadStorage,
};

#[derive(Clone)]
pub struct AdminState {
    pub db: Arc<PostgresRepositories>,
    pub cache: Arc<ResponseCache>,
    pub cache_warm_debouncer: Arc<CacheWarmDebouncer>,
    pub chrome: Arc<AdminChromeService>,
    pub dashboard: Arc<AdminDashboardService>,
    pub posts: Arc<AdminPostService>,
    pub pages: Arc<AdminPageService>,
    pub tags: Arc<AdminTagService>,
    pub navigation: Arc<AdminNavigationService>,
    pub settings: Arc<AdminSettingsService>,
    pub uploads: Arc<AdminUploadService>,
    pub upload_storage: Arc<UploadStorage>,
    pub upload_limit_bytes: u64,
    pub jobs: Arc<AdminJobService>,
    pub audit: Arc<AdminAuditService>,
    pub api_keys: Arc<ApiKeyService>,
    pub snapshots: Arc<AdminSnapshotService>,
}
