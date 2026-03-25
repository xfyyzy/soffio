use async_trait::async_trait;
use serde::Serialize;

use crate::application::pagination::{CursorPage, PageRequest};
use crate::domain::types::SnapshotEntityType;

use super::RepoError;

pub type SnapshotCursor = crate::application::pagination::SnapshotCursor;

#[derive(Debug, Clone, Default)]
pub struct SnapshotFilter {
    pub entity_type: Option<SnapshotEntityType>,
    pub entity_id: Option<uuid::Uuid>,
    pub search: Option<String>,
    pub month: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SnapshotMonthCount {
    pub key: String,
    pub label: String,
    pub count: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct SnapshotRecord {
    pub id: uuid::Uuid,
    pub entity_type: SnapshotEntityType,
    pub entity_id: uuid::Uuid,
    pub version: i32,
    pub description: Option<String>,
    pub schema_version: i64,
    pub content: serde_json::Value,
    pub created_at: time::OffsetDateTime,
}

#[async_trait]
pub trait SnapshotsRepo: Send + Sync {
    async fn create(&self, record: SnapshotRecord) -> Result<(), RepoError>;

    async fn list_snapshots(
        &self,
        filter: &SnapshotFilter,
        page: PageRequest<SnapshotCursor>,
    ) -> Result<CursorPage<SnapshotRecord>, RepoError>;

    async fn count_snapshots(&self, filter: &SnapshotFilter) -> Result<u64, RepoError>;

    async fn find_snapshot(&self, id: uuid::Uuid) -> Result<Option<SnapshotRecord>, RepoError>;

    async fn latest_snapshot(
        &self,
        entity_type: SnapshotEntityType,
        entity_id: uuid::Uuid,
    ) -> Result<Option<SnapshotRecord>, RepoError>;

    /// Highest applied migration version from `_sqlx_migrations`.
    async fn current_schema_version(&self) -> Result<i64, RepoError>;

    async fn month_counts(
        &self,
        filter: &SnapshotFilter,
    ) -> Result<Vec<SnapshotMonthCount>, RepoError>;

    async fn update_description(
        &self,
        id: uuid::Uuid,
        description: Option<String>,
    ) -> Result<Option<SnapshotRecord>, RepoError>;

    async fn delete_snapshot(&self, id: uuid::Uuid) -> Result<Option<SnapshotRecord>, RepoError>;
}
