use async_trait::async_trait;
use sqlx::QueryBuilder;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::application::pagination::{CursorPage, PageRequest, SnapshotCursor};
use crate::application::repos::{RepoError, SnapshotFilter, SnapshotRecord, SnapshotsRepo};
use crate::domain::types::SnapshotEntityType;

use super::{PostgresRepositories, map_sqlx_error};

#[derive(sqlx::FromRow)]
struct SnapshotRow {
    id: Uuid,
    entity_type: SnapshotEntityType,
    entity_id: Uuid,
    version: i32,
    description: Option<String>,
    schema_version: i64,
    content: serde_json::Value,
    created_at: OffsetDateTime,
}

impl From<SnapshotRow> for SnapshotRecord {
    fn from(row: SnapshotRow) -> Self {
        Self {
            id: row.id,
            entity_type: row.entity_type,
            entity_id: row.entity_id,
            version: row.version,
            description: row.description,
            schema_version: row.schema_version,
            content: row.content,
            created_at: row.created_at,
        }
    }
}

#[async_trait]
impl SnapshotsRepo for PostgresRepositories {
    async fn create(&self, record: SnapshotRecord) -> Result<(), RepoError> {
        sqlx::query!(
            r#"
            INSERT INTO snapshots (
                id, entity_type, entity_id, version, description, schema_version, content, created_at
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            "#,
            record.id,
            record.entity_type as SnapshotEntityType,
            record.entity_id,
            record.version,
            record.description,
            record.schema_version,
            record.content,
            record.created_at
        )
        .execute(self.pool())
        .await
        .map_err(map_sqlx_error)?;

        Ok(())
    }

    async fn list_snapshots(
        &self,
        filter: &SnapshotFilter,
        page: PageRequest<SnapshotCursor>,
    ) -> Result<CursorPage<SnapshotRecord>, RepoError> {
        let limit = page.limit.clamp(1, 100) as i64;
        let mut qb = QueryBuilder::new(
            "SELECT id, entity_type, entity_id, version, description, schema_version, content, created_at FROM snapshots WHERE 1=1 ",
        );

        if let Some(entity_type) = filter.entity_type {
            qb.push(" AND entity_type = ");
            qb.push_bind(entity_type);
        }

        if let Some(entity_id) = filter.entity_id {
            qb.push(" AND entity_id = ");
            qb.push_bind(entity_id);
        }

        if let Some(search) = filter.search.as_ref() {
            qb.push(" AND (description ILIKE ");
            qb.push_bind(format!("%{}%", search));
            qb.push(")");
        }

        if let Some(month) = filter.month.as_ref() {
            qb.push(" AND to_char(created_at, 'YYYY-MM') = ");
            qb.push_bind(month);
        }

        if let Some(cursor) = page.cursor {
            qb.push(" AND (created_at, id) < (");
            qb.push_bind(cursor.created_at());
            qb.push(", ");
            qb.push_bind(cursor.id());
            qb.push(")");
        }

        qb.push(" ORDER BY created_at DESC, id DESC LIMIT ");
        qb.push_bind(limit + 1);

        let mut rows = qb
            .build_query_as::<SnapshotRow>()
            .fetch_all(self.pool())
            .await
            .map_err(map_sqlx_error)?;

        let next_cursor = if (rows.len() as i64) > limit {
            let overflow = rows.pop().expect("overflow row exists when len > limit");
            Some(SnapshotCursor::new(overflow.created_at, overflow.id).encode())
        } else {
            None
        };

        let records = rows.into_iter().map(SnapshotRecord::from).collect();
        Ok(CursorPage::new(records, next_cursor))
    }

    async fn count_snapshots(&self, filter: &SnapshotFilter) -> Result<u64, RepoError> {
        let mut qb = QueryBuilder::new("SELECT COUNT(*) FROM snapshots WHERE 1=1 ");

        if let Some(entity_type) = filter.entity_type {
            qb.push(" AND entity_type = ");
            qb.push_bind(entity_type);
        }

        if let Some(entity_id) = filter.entity_id {
            qb.push(" AND entity_id = ");
            qb.push_bind(entity_id);
        }

        if let Some(search) = filter.search.as_ref() {
            qb.push(" AND (description ILIKE ");
            qb.push_bind(format!("%{}%", search));
            qb.push(")");
        }

        if let Some(month) = filter.month.as_ref() {
            qb.push(" AND to_char(created_at, 'YYYY-MM') = ");
            qb.push_bind(month);
        }

        let count: i64 = qb
            .build_query_scalar()
            .fetch_one(self.pool())
            .await
            .map_err(map_sqlx_error)?;

        Ok(count as u64)
    }

    async fn find_snapshot(&self, id: Uuid) -> Result<Option<SnapshotRecord>, RepoError> {
        let row: Option<SnapshotRow> = sqlx::query_as!(
            SnapshotRow,
            r#"
            SELECT id, entity_type AS "entity_type: SnapshotEntityType", entity_id, version, description, schema_version, content, created_at
            FROM snapshots WHERE id = $1
            "#,
            id
        )
        .fetch_optional(self.pool())
        .await
        .map_err(map_sqlx_error)?;

        Ok(row.map(SnapshotRecord::from))
    }

    async fn latest_snapshot(
        &self,
        entity_type: SnapshotEntityType,
        entity_id: Uuid,
    ) -> Result<Option<SnapshotRecord>, RepoError> {
        let row: Option<SnapshotRow> = sqlx::query_as!(
            SnapshotRow,
            r#"
            SELECT id, entity_type AS "entity_type: SnapshotEntityType", entity_id, version, description, schema_version, content, created_at
            FROM snapshots
            WHERE entity_type = $1 AND entity_id = $2
            ORDER BY version DESC
            LIMIT 1
            "#,
            entity_type as SnapshotEntityType,
            entity_id
        )
        .fetch_optional(self.pool())
        .await
        .map_err(map_sqlx_error)?;

        Ok(row.map(SnapshotRecord::from))
    }

    async fn current_schema_version(&self) -> Result<i64, RepoError> {
        let version: Option<i64> =
            sqlx::query_scalar(r#"SELECT MAX(version) FROM _sqlx_migrations"#)
                .fetch_one(self.pool())
                .await
                .map_err(map_sqlx_error)?;

        Ok(version.unwrap_or(0))
    }

    async fn month_counts(
        &self,
        filter: &SnapshotFilter,
    ) -> Result<Vec<crate::application::repos::SnapshotMonthCount>, RepoError> {
        #[derive(sqlx::FromRow)]
        struct MonthRow {
            bucket: OffsetDateTime,
            count: i64,
        }

        let mut qb = QueryBuilder::new(
            "SELECT date_trunc('month', created_at) AS bucket, COUNT(*) AS count FROM snapshots WHERE 1=1 ",
        );

        if let Some(entity_type) = filter.entity_type {
            qb.push(" AND entity_type = ");
            qb.push_bind(entity_type);
        }

        if let Some(entity_id) = filter.entity_id {
            qb.push(" AND entity_id = ");
            qb.push_bind(entity_id);
        }

        if let Some(search) = filter.search.as_ref() {
            qb.push(" AND (description ILIKE ");
            qb.push_bind(format!("%{}%", search));
            qb.push(")");
        }

        qb.push(" GROUP BY bucket ORDER BY bucket DESC ");

        let rows: Vec<MonthRow> = qb
            .build_query_as::<MonthRow>()
            .fetch_all(self.pool())
            .await
            .map_err(map_sqlx_error)?;

        let mut counts = Vec::with_capacity(rows.len());
        for row in rows {
            let date = row.bucket.date();
            let key = crate::domain::posts::month_key_for(date);
            let label = crate::domain::posts::month_label_for(date);
            counts.push(crate::application::repos::SnapshotMonthCount {
                key,
                label,
                count: row.count as usize,
            });
        }

        Ok(counts)
    }

    async fn update_description(
        &self,
        id: Uuid,
        description: Option<String>,
    ) -> Result<Option<SnapshotRecord>, RepoError> {
        let row: Option<SnapshotRow> = sqlx::query_as::<_, SnapshotRow>(
            r#"
            UPDATE snapshots
            SET description = $2
            WHERE id = $1
            RETURNING id, entity_type AS "entity_type: SnapshotEntityType", entity_id, version, description, schema_version, content, created_at
            "#,
        )
        .bind(id)
        .bind(description)
        .fetch_optional(self.pool())
        .await
        .map_err(map_sqlx_error)?;

        Ok(row.map(SnapshotRecord::from))
    }

    async fn delete_snapshot(&self, id: Uuid) -> Result<Option<SnapshotRecord>, RepoError> {
        let row: Option<SnapshotRow> = sqlx::query_as::<_, SnapshotRow>(
            r#"
            DELETE FROM snapshots
            WHERE id = $1
            RETURNING id, entity_type AS "entity_type: SnapshotEntityType", entity_id, version, description, schema_version, content, created_at
            "#,
        )
        .bind(id)
        .fetch_optional(self.pool())
        .await
        .map_err(map_sqlx_error)?;

        Ok(row.map(SnapshotRecord::from))
    }
}
