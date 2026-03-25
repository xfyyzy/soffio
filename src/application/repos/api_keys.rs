use async_trait::async_trait;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::application::pagination::ApiKeyCursor;
use crate::domain::api_keys::{ApiKeyRecord, ApiScope};

use super::RepoError;

#[derive(Debug, Clone)]
pub struct CreateApiKeyParams {
    pub name: String,
    pub description: Option<String>,
    pub prefix: String,
    pub hashed_secret: Vec<u8>,
    pub scopes: Vec<ApiScope>,
    pub expires_in: Option<time::Duration>,
    pub expires_at: Option<OffsetDateTime>,
    pub created_by: String,
}

#[derive(Debug, Clone)]
pub struct UpdateApiKeySecretParams {
    pub id: Uuid,
    pub new_prefix: String,
    pub new_hashed_secret: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct UpdateApiKeyMetadataParams {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub scopes: Vec<ApiScope>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApiKeyStatusFilter {
    Active,
    Revoked,
    Expired,
}

#[derive(Debug, Clone, Default)]
pub struct ApiKeyQueryFilter {
    pub status: Option<ApiKeyStatusFilter>,
    pub scope: Option<ApiScope>,
    pub search: Option<String>,
}

#[derive(Debug, Clone, Copy)]
pub struct ApiKeyPageRequest {
    pub limit: u32,
    pub cursor: Option<ApiKeyCursor>,
}

#[derive(Debug, Clone)]
pub struct ApiKeyListPage {
    pub items: Vec<ApiKeyRecord>,
    pub total: u64,
    pub active: u64,
    pub revoked: u64,
    pub expired: u64,
    pub next_cursor: Option<ApiKeyCursor>,
    pub scope_counts: Vec<(ApiScope, u64)>,
}

#[async_trait]
pub trait ApiKeysRepo: Send + Sync {
    async fn create_key(&self, params: CreateApiKeyParams) -> Result<ApiKeyRecord, RepoError>;

    async fn list_keys(
        &self,
        filter: &ApiKeyQueryFilter,
        page: ApiKeyPageRequest,
    ) -> Result<ApiKeyListPage, RepoError>;

    async fn find_by_prefix(&self, prefix: &str) -> Result<Option<ApiKeyRecord>, RepoError>;

    async fn find_by_id(&self, id: Uuid) -> Result<Option<ApiKeyRecord>, RepoError>;

    async fn revoke_key(&self, id: Uuid, revoked_at: OffsetDateTime) -> Result<(), RepoError>;

    async fn delete_key(&self, id: Uuid) -> Result<bool, RepoError>;

    async fn expire_keys(&self) -> Result<u64, RepoError>;

    async fn update_secret(
        &self,
        params: UpdateApiKeySecretParams,
    ) -> Result<ApiKeyRecord, RepoError>;

    async fn update_metadata(
        &self,
        params: UpdateApiKeyMetadataParams,
    ) -> Result<ApiKeyRecord, RepoError>;

    async fn update_last_used(
        &self,
        id: Uuid,
        last_used_at: OffsetDateTime,
    ) -> Result<(), RepoError>;
}
