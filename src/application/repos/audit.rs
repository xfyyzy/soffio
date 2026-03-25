use async_trait::async_trait;
use uuid::Uuid;

use crate::application::pagination::{AuditCursor, CursorPage, PageRequest};
use crate::domain::entities::AuditLogRecord;

use super::RepoError;

#[derive(Debug, Clone, Default)]
pub struct AuditQueryFilter {
    pub actor: Option<String>,
    pub action: Option<String>,
    pub entity_type: Option<String>,
    pub search: Option<String>,
}

/// Count of audit logs by entity type.
#[derive(Debug, Clone)]
pub struct AuditEntityTypeCount {
    pub entity_type: String,
    pub count: u64,
}

/// Count of audit logs by actor.
#[derive(Debug, Clone)]
pub struct AuditActorCount {
    pub actor: String,
    pub count: u64,
}

/// Count of audit logs by action.
#[derive(Debug, Clone)]
pub struct AuditActionCount {
    pub action: String,
    pub count: u64,
}

#[async_trait]
pub trait AuditRepo: Send + Sync {
    async fn append_log(&self, record: AuditLogRecord) -> Result<(), RepoError>;
    async fn list_recent(&self, limit: u32) -> Result<Vec<AuditLogRecord>, RepoError>;
    async fn list_filtered(
        &self,
        page: PageRequest<AuditCursor>,
        filter: &AuditQueryFilter,
    ) -> Result<CursorPage<AuditLogRecord>, RepoError>;
    async fn count_filtered(&self, filter: &AuditQueryFilter) -> Result<u64, RepoError>;
    async fn list_entity_type_counts(
        &self,
        filter: &AuditQueryFilter,
    ) -> Result<Vec<AuditEntityTypeCount>, RepoError>;
    async fn list_distinct_actors(
        &self,
        filter: &AuditQueryFilter,
    ) -> Result<Vec<AuditActorCount>, RepoError>;
    async fn list_distinct_actions(
        &self,
        filter: &AuditQueryFilter,
    ) -> Result<Vec<AuditActionCount>, RepoError>;
    async fn find_by_id(&self, id: Uuid) -> Result<Option<AuditLogRecord>, RepoError>;
}
