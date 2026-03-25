use async_trait::async_trait;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::application::pagination::{CursorPage, PageRequest, UploadCursor};
use crate::domain::entities::UploadRecord;

use super::RepoError;

#[derive(Debug, Clone, Default)]
pub struct UploadQueryFilter {
    pub content_type: Option<String>,
    pub month: Option<String>,
    pub search: Option<String>,
}

#[derive(Debug, Clone)]
pub struct UploadContentTypeCount {
    pub content_type: String,
    pub count: u64,
}

#[derive(Debug, Clone)]
pub struct UploadMonthCount {
    pub key: String,
    pub label: String,
    pub count: u64,
}

#[async_trait]
pub trait UploadsRepo: Send + Sync {
    async fn insert_upload(&self, record: UploadRecord) -> Result<(), RepoError>;
    async fn find_upload(&self, id: Uuid) -> Result<Option<UploadRecord>, RepoError>;
    async fn list_recent(
        &self,
        limit: u32,
        before: Option<OffsetDateTime>,
    ) -> Result<Vec<UploadRecord>, RepoError>;
    async fn list_uploads(
        &self,
        filter: &UploadQueryFilter,
        page: PageRequest<UploadCursor>,
    ) -> Result<CursorPage<UploadRecord>, RepoError>;
    async fn count_uploads(&self, filter: &UploadQueryFilter) -> Result<u64, RepoError>;
    async fn sum_upload_sizes(&self, filter: &UploadQueryFilter) -> Result<u64, RepoError>;
    async fn month_counts(
        &self,
        filter: &UploadQueryFilter,
    ) -> Result<Vec<UploadMonthCount>, RepoError>;
    async fn content_type_counts(
        &self,
        filter: &UploadQueryFilter,
    ) -> Result<Vec<UploadContentTypeCount>, RepoError>;
    async fn delete_upload(&self, id: Uuid) -> Result<(), RepoError>;
}
