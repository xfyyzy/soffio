use std::sync::Arc;

use serde::Serialize;
use thiserror::Error;
use uuid::Uuid;

use crate::application::admin::audit::AdminAuditService;
use crate::application::pagination::{CursorPage, PageRequest, UploadCursor};
use crate::application::repos::{
    RepoError, UploadContentTypeCount, UploadMonthCount, UploadQueryFilter, UploadsRepo,
};
use crate::domain::entities::UploadRecord;

#[derive(Debug, Error)]
pub enum AdminUploadError {
    #[error("upload not found")]
    NotFound,
    #[error(transparent)]
    Repo(#[from] RepoError),
}

#[derive(Clone)]
pub struct AdminUploadService {
    repo: Arc<dyn UploadsRepo>,
    audit: AdminAuditService,
}

impl AdminUploadService {
    pub fn new(repo: Arc<dyn UploadsRepo>, audit: AdminAuditService) -> Self {
        Self { repo, audit }
    }

    pub async fn register_upload(
        &self,
        actor: &str,
        record: UploadRecord,
    ) -> Result<(), AdminUploadError> {
        let snapshot = UploadSnapshot::from(&record);
        self.repo.insert_upload(record.clone()).await?;
        self.audit
            .record(
                actor,
                "upload.register",
                "upload",
                Some(&record.id.to_string()),
                Some(&snapshot),
            )
            .await?;
        Ok(())
    }

    pub async fn find_upload(&self, id: Uuid) -> Result<Option<UploadRecord>, AdminUploadError> {
        self.repo
            .find_upload(id)
            .await
            .map_err(AdminUploadError::from)
    }

    pub async fn list(
        &self,
        filter: &UploadQueryFilter,
        page: PageRequest<UploadCursor>,
    ) -> Result<CursorPage<UploadRecord>, AdminUploadError> {
        self.repo
            .list_uploads(filter, page)
            .await
            .map_err(AdminUploadError::from)
    }

    pub async fn count(&self, filter: &UploadQueryFilter) -> Result<u64, AdminUploadError> {
        self.repo
            .count_uploads(filter)
            .await
            .map_err(AdminUploadError::from)
    }

    pub async fn month_counts(
        &self,
        filter: &UploadQueryFilter,
    ) -> Result<Vec<UploadMonthCount>, AdminUploadError> {
        self.repo
            .month_counts(filter)
            .await
            .map_err(AdminUploadError::from)
    }

    pub async fn content_type_counts(
        &self,
        filter: &UploadQueryFilter,
    ) -> Result<Vec<UploadContentTypeCount>, AdminUploadError> {
        self.repo
            .content_type_counts(filter)
            .await
            .map_err(AdminUploadError::from)
    }

    pub async fn delete_upload(
        &self,
        actor: &str,
        id: Uuid,
    ) -> Result<UploadRecord, AdminUploadError> {
        let record = self
            .repo
            .find_upload(id)
            .await?
            .ok_or(AdminUploadError::NotFound)?;

        self.repo.delete_upload(id).await?;

        let snapshot = UploadSnapshot::from(&record);
        self.audit
            .record(
                actor,
                "upload.delete",
                "upload",
                Some(&record.id.to_string()),
                Some(&snapshot),
            )
            .await?;

        Ok(record)
    }
}

#[derive(Debug, Serialize)]
struct UploadSnapshot<'a> {
    filename: &'a str,
    content_type: &'a str,
    size_bytes: i64,
    checksum: &'a str,
}

impl<'a> From<&'a UploadRecord> for UploadSnapshot<'a> {
    fn from(record: &'a UploadRecord) -> Self {
        Self {
            filename: record.filename.as_str(),
            content_type: record.content_type.as_str(),
            size_bytes: record.size_bytes,
            checksum: record.checksum.as_str(),
        }
    }
}
