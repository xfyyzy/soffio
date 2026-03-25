use async_trait::async_trait;

use crate::domain::entities::SiteSettingsRecord;

use super::RepoError;

#[async_trait]
pub trait SettingsRepo: Send + Sync {
    async fn load_site_settings(&self) -> Result<SiteSettingsRecord, RepoError>;
    async fn upsert_site_settings(&self, settings: SiteSettingsRecord) -> Result<(), RepoError>;
}
