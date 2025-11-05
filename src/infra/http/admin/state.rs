use std::sync::Arc;

use crate::application::admin::{
    audit::AdminAuditService, chrome::AdminChromeService, dashboard::AdminDashboardService,
    jobs::AdminJobService, navigation::AdminNavigationService, pages::AdminPageService,
    posts::AdminPostService, settings::AdminSettingsService, tags::AdminTagService,
    uploads::AdminUploadService,
};
use crate::infra::{cache::ResponseCache, db::PostgresRepositories, uploads::UploadStorage};

#[derive(Clone)]
pub struct AdminState {
    pub db: Arc<PostgresRepositories>,
    pub cache: Arc<ResponseCache>,
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
}
