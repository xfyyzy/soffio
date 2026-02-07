use async_trait::async_trait;
use serde_json::Value as JsonValue;
use sqlx::{Postgres, QueryBuilder};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{
    application::{
        pagination::{CursorPage, PageRequest, UploadCursor},
        repos::{
            RepoError, UploadContentTypeCount, UploadMonthCount, UploadQueryFilter, UploadsRepo,
        },
    },
    domain::{
        entities::UploadRecord,
        uploads::{self},
    },
};

use super::{PostgresRepositories, map_sqlx_error};

#[derive(sqlx::FromRow)]
struct UploadRow {
    id: Uuid,
    filename: String,
    content_type: String,
    size_bytes: i64,
    checksum: String,
    stored_path: String,
    metadata: JsonValue,
    created_at: OffsetDateTime,
}

impl From<UploadRow> for UploadRecord {
    fn from(row: UploadRow) -> Self {
        let metadata = serde_json::from_value(row.metadata).unwrap_or_default();
        Self {
            id: row.id,
            filename: row.filename,
            content_type: row.content_type,
            size_bytes: row.size_bytes,
            checksum: row.checksum,
            stored_path: row.stored_path,
            metadata,
            created_at: row.created_at,
        }
    }
}

#[async_trait]
impl UploadsRepo for PostgresRepositories {
    async fn insert_upload(&self, record: UploadRecord) -> Result<(), RepoError> {
        let metadata_json = serde_json::to_value(&record.metadata).expect("metadata serializable");

        sqlx::query!(
            r#"
            INSERT INTO uploads (id, filename, content_type, size_bytes, checksum, stored_path, metadata, created_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            "#,
            record.id,
            record.filename,
            record.content_type,
            record.size_bytes,
            record.checksum,
            record.stored_path,
            metadata_json,
            record.created_at
        )
        .execute(self.pool())
        .await
        .map_err(|err| match err {
            sqlx::Error::Database(db_err) => {
                if let Some(constraint) = db_err.constraint() {
                    RepoError::Duplicate {
                        constraint: constraint.to_owned(),
                    }
                } else {
                    RepoError::from_persistence(db_err)
                }
            }
            other => RepoError::from_persistence(other),
        })?;

        Ok(())
    }

    async fn find_upload(&self, id: Uuid) -> Result<Option<UploadRecord>, RepoError> {
        let row = sqlx::query_as!(
            UploadRow,
            r#"
            SELECT id, filename, content_type, size_bytes, checksum, stored_path, metadata, created_at
            FROM uploads
            WHERE id = $1
            "#,
            id
        )
        .fetch_optional(self.pool())
        .await
        .map_err(map_sqlx_error)?;

        Ok(row.map(UploadRecord::from))
    }

    async fn list_recent(
        &self,
        limit: u32,
        before: Option<OffsetDateTime>,
    ) -> Result<Vec<UploadRecord>, RepoError> {
        let limit = limit.clamp(1, 200) as i64;
        let mut qb = QueryBuilder::new(
            "SELECT id, filename, content_type, size_bytes, checksum, stored_path, metadata, created_at \
             FROM uploads",
        );

        if let Some(before) = before {
            qb.push(" WHERE created_at < ");
            qb.push_bind(before);
        }

        qb.push(" ORDER BY created_at DESC, id DESC");
        qb.push(" LIMIT ");
        qb.push_bind(limit);

        let rows = qb
            .build_query_as::<UploadRow>()
            .fetch_all(self.pool())
            .await
            .map_err(map_sqlx_error)?;

        Ok(rows.into_iter().map(UploadRecord::from).collect())
    }

    async fn list_uploads(
        &self,
        filter: &UploadQueryFilter,
        page: PageRequest<UploadCursor>,
    ) -> Result<CursorPage<UploadRecord>, RepoError> {
        let limit = page.limit.clamp(1, 200) as i64;
        let mut qb = QueryBuilder::new(
            "SELECT id, filename, content_type, size_bytes, checksum, stored_path, metadata, created_at \
             FROM uploads WHERE 1=1 ",
        );

        apply_filter(&mut qb, filter);

        if let Some(cursor) = page.cursor {
            qb.push(" AND (created_at, id) < (");
            qb.push_bind(cursor.created_at());
            qb.push(", ");
            qb.push_bind(cursor.id());
            qb.push(") ");
        }

        qb.push(" ORDER BY created_at DESC, id DESC ");
        qb.push(" LIMIT ");
        qb.push_bind(limit + 1);

        let mut rows = qb
            .build_query_as::<UploadRow>()
            .fetch_all(self.pool())
            .await
            .map_err(map_sqlx_error)?;

        let has_more = (rows.len() as i64) > limit;
        if has_more {
            rows.pop();
        }

        let next_cursor = if has_more {
            let row = rows
                .last()
                .expect("cursor computation requires at least one row");
            let cursor = UploadCursor::new(row.created_at, row.id);
            Some(cursor.encode())
        } else {
            None
        };

        let records = rows.into_iter().map(UploadRecord::from).collect();

        Ok(CursorPage::new(records, next_cursor))
    }

    async fn count_uploads(&self, filter: &UploadQueryFilter) -> Result<u64, RepoError> {
        let mut qb = QueryBuilder::new("SELECT COUNT(*) FROM uploads WHERE 1=1 ");
        apply_filter(&mut qb, filter);

        let count: i64 = qb
            .build_query_scalar()
            .fetch_one(self.pool())
            .await
            .map_err(map_sqlx_error)?;

        PostgresRepositories::convert_count(count)
    }

    async fn sum_upload_sizes(&self, filter: &UploadQueryFilter) -> Result<u64, RepoError> {
        let mut qb = QueryBuilder::new(
            "SELECT COALESCE(SUM(size_bytes), 0)::BIGINT FROM uploads WHERE 1=1 ",
        );
        apply_filter(&mut qb, filter);

        let total: i64 = qb
            .build_query_scalar()
            .fetch_one(self.pool())
            .await
            .map_err(map_sqlx_error)?;

        PostgresRepositories::convert_count(total)
    }

    async fn month_counts(
        &self,
        filter: &UploadQueryFilter,
    ) -> Result<Vec<UploadMonthCount>, RepoError> {
        #[derive(sqlx::FromRow)]
        struct MonthRow {
            bucket: OffsetDateTime,
            count: i64,
        }

        let mut qb = QueryBuilder::new(
            "SELECT date_trunc('month', created_at) AS bucket, COUNT(*) AS count \
             FROM uploads WHERE 1=1 ",
        );

        apply_filter(&mut qb, filter);

        qb.push(" GROUP BY bucket ORDER BY bucket DESC ");

        let rows = qb
            .build_query_as::<MonthRow>()
            .fetch_all(self.pool())
            .await
            .map_err(map_sqlx_error)?;

        let mut counts = Vec::with_capacity(rows.len());
        for row in rows {
            counts.push(UploadMonthCount {
                key: uploads::month_key_for(row.bucket),
                label: uploads::month_label_for(row.bucket),
                count: PostgresRepositories::convert_count(row.count)?,
            });
        }

        Ok(counts)
    }

    async fn content_type_counts(
        &self,
        filter: &UploadQueryFilter,
    ) -> Result<Vec<UploadContentTypeCount>, RepoError> {
        #[derive(sqlx::FromRow)]
        struct TypeRow {
            content_type: String,
            count: i64,
        }

        let mut qb =
            QueryBuilder::new("SELECT content_type, COUNT(*) AS count FROM uploads WHERE 1=1 ");

        apply_filter(&mut qb, filter);

        qb.push(" GROUP BY content_type ");
        qb.push(" ORDER BY count DESC, content_type ASC ");

        let rows = qb
            .build_query_as::<TypeRow>()
            .fetch_all(self.pool())
            .await
            .map_err(map_sqlx_error)?;

        let mut counts = Vec::with_capacity(rows.len());
        for row in rows {
            counts.push(UploadContentTypeCount {
                content_type: row.content_type,
                count: PostgresRepositories::convert_count(row.count)?,
            });
        }

        Ok(counts)
    }

    async fn delete_upload(&self, id: Uuid) -> Result<(), RepoError> {
        sqlx::query!(
            r#"
            DELETE FROM uploads
            WHERE id = $1
            "#,
            id
        )
        .execute(self.pool())
        .await
        .map_err(map_sqlx_error)?;

        Ok(())
    }
}

fn apply_filter<'q>(qb: &mut QueryBuilder<'q, Postgres>, filter: &'q UploadQueryFilter) {
    if let Some(content_type) = filter.content_type.as_ref() {
        qb.push(" AND content_type = ");
        qb.push_bind(content_type);
        qb.push(' ');
    }

    if let Some(month) = filter.month.as_ref() {
        qb.push(" AND to_char(created_at, 'YYYY-MM') = ");
        qb.push_bind(month);
        qb.push(' ');
    }

    if let Some(search) = filter.search.as_ref() {
        qb.push(" AND filename ILIKE ");
        qb.push_bind(format!("%{search}%"));
        qb.push(' ');
    }
}
