use std::sync::Arc;

use serde::Serialize;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::application::pagination::{AuditCursor, CursorPage, PageRequest};
use crate::application::repos::{AuditQueryFilter, AuditRepo, RepoError};
use crate::domain::entities::AuditLogRecord;

/// Thin wrapper around the audit repository to simplify logging admin actions.
#[derive(Clone)]
pub struct AdminAuditService {
    repo: Arc<dyn AuditRepo>,
}

impl AdminAuditService {
    pub fn new(repo: Arc<dyn AuditRepo>) -> Self {
        Self { repo }
    }

    pub async fn record<S>(
        &self,
        actor: &str,
        action: &str,
        entity_type: &str,
        entity_id: Option<&str>,
        payload: Option<&S>,
    ) -> Result<(), RepoError>
    where
        S: Serialize,
    {
        let payload_text = match payload {
            Some(value) => Some(serde_json::to_string(value).map_err(RepoError::from_persistence)?),
            None => None,
        };

        let record = AuditLogRecord {
            id: Uuid::new_v4(),
            actor: actor.to_string(),
            action: action.to_string(),
            entity_type: entity_type.to_string(),
            entity_id: entity_id.map(|value| value.to_string()),
            payload_text,
            created_at: OffsetDateTime::now_utc(),
        };

        self.repo.append_log(record).await
    }

    pub async fn list_recent(&self, limit: u32) -> Result<Vec<AuditLogRecord>, RepoError> {
        let page = PageRequest::new(limit, None);
        let records = self
            .repo
            .list_filtered(page, &AuditQueryFilter::default())
            .await?
            .items;
        Ok(records)
    }

    pub async fn list_filtered(
        &self,
        page: PageRequest<AuditCursor>,
        filter: &AuditQueryFilter,
    ) -> Result<CursorPage<AuditLogRecord>, RepoError> {
        self.repo.list_filtered(page, filter).await
    }
}
