use async_trait::async_trait;
use sqlx::QueryBuilder;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{
    application::pagination::{AuditCursor, CursorPage, PageRequest},
    application::repos::{AuditQueryFilter, AuditRepo, RepoError},
    domain::entities::AuditLogRecord,
};

use super::{PostgresRepositories, map_sqlx_error};

#[derive(sqlx::FromRow)]
struct AuditRow {
    id: Uuid,
    actor: String,
    action: String,
    entity_type: String,
    entity_id: Option<String>,
    payload_text: Option<String>,
    created_at: OffsetDateTime,
}

impl From<AuditRow> for AuditLogRecord {
    fn from(row: AuditRow) -> Self {
        Self {
            id: row.id,
            actor: row.actor,
            action: row.action,
            entity_type: row.entity_type,
            entity_id: row.entity_id,
            payload_text: row.payload_text,
            created_at: row.created_at,
        }
    }
}

#[async_trait]
impl AuditRepo for PostgresRepositories {
    async fn append_log(&self, record: AuditLogRecord) -> Result<(), RepoError> {
        sqlx::query!(
            r#"
            INSERT INTO audit_logs (id, actor, action, entity_type, entity_id, payload_text, created_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            "#,
            record.id,
            record.actor,
            record.action,
            record.entity_type,
            record.entity_id,
            record.payload_text,
            record.created_at
        )
        .execute(self.pool())
        .await
        .map_err(map_sqlx_error)?;

        Ok(())
    }

    async fn list_recent(&self, limit: u32) -> Result<Vec<AuditLogRecord>, RepoError> {
        let filter = AuditQueryFilter::default();
        let page = PageRequest::new(limit, None);
        let result = self.list_filtered(page, &filter).await?;
        Ok(result.items)
    }

    async fn list_filtered(
        &self,
        page: PageRequest<AuditCursor>,
        filter: &AuditQueryFilter,
    ) -> Result<CursorPage<AuditLogRecord>, RepoError> {
        let limit = page.limit.clamp(1, 200);
        let mut qb = QueryBuilder::new(
            "SELECT id, actor, action, entity_type, entity_id, payload_text, created_at \
             FROM audit_logs WHERE 1=1 ",
        );

        if let Some(actor) = filter.actor.as_ref() {
            let pattern = format!("%{}%", actor);
            qb.push(" AND actor ILIKE ");
            qb.push_bind(pattern);
        }

        if let Some(action) = filter.action.as_ref() {
            let pattern = format!("%{}%", action);
            qb.push(" AND action ILIKE ");
            qb.push_bind(pattern);
        }

        if let Some(entity_type) = filter.entity_type.as_ref() {
            let pattern = format!("%{}%", entity_type);
            qb.push(" AND entity_type ILIKE ");
            qb.push_bind(pattern);
        }

        if let Some(search) = filter.search.as_ref() {
            let pattern = format!("%{}%", search);
            qb.push(" AND (COALESCE(entity_id, '') ILIKE ");
            qb.push_bind(pattern.clone());
            qb.push(" OR COALESCE(payload_text, '') ILIKE ");
            qb.push_bind(pattern);
            qb.push(")");
        }

        if let Some(cursor) = page.cursor {
            qb.push(" AND (");
            qb.push("created_at < ");
            qb.push_bind(cursor.created_at());
            qb.push(" OR (created_at = ");
            qb.push_bind(cursor.created_at());
            qb.push(" AND id < ");
            qb.push_bind(cursor.id());
            qb.push("))");
        }

        qb.push(" ORDER BY created_at DESC, id DESC LIMIT ");
        qb.push_bind(limit as i64);

        let rows = qb
            .build_query_as::<AuditRow>()
            .fetch_all(self.pool())
            .await
            .map_err(map_sqlx_error)?;

        let records: Vec<AuditLogRecord> = rows.into_iter().map(AuditLogRecord::from).collect();
        let next_cursor = if records.len() as u32 == limit {
            records
                .last()
                .map(|entry| AuditCursor::new(entry.created_at, entry.id).encode())
        } else {
            None
        };

        Ok(CursorPage::new(records, next_cursor))
    }
}
