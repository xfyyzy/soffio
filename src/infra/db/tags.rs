use async_trait::async_trait;
use sqlx::{Postgres, QueryBuilder};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{
    application::pagination::{CursorPage, PageRequest, TagCursor},
    application::repos::{
        CreateTagParams, RepoError, TagListRecord, TagQueryFilter, TagWithCount, TagsRepo,
        TagsWriteRepo, UpdateTagParams,
    },
    domain::entities::TagRecord,
};

use super::PostgresRepositories;

const TAG_PRIMARY_TIME_EXPR: &str = "COALESCE(t.updated_at, t.created_at)";

#[derive(sqlx::FromRow)]
struct TagRow {
    id: Uuid,
    slug: String,
    name: String,
    description: Option<String>,
    pinned: bool,
    created_at: OffsetDateTime,
    updated_at: OffsetDateTime,
}

impl From<TagRow> for TagRecord {
    fn from(row: TagRow) -> Self {
        Self {
            id: row.id,
            slug: row.slug,
            name: row.name,
            description: row.description,
            pinned: row.pinned,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

#[derive(sqlx::FromRow)]
struct TagListRow {
    id: Uuid,
    slug: String,
    name: String,
    description: Option<String>,
    pinned: bool,
    created_at: OffsetDateTime,
    updated_at: Option<OffsetDateTime>,
    primary_time: OffsetDateTime,
    usage_count: i64,
}

#[async_trait]
impl TagsRepo for PostgresRepositories {
    async fn list_all(&self) -> Result<Vec<TagRecord>, RepoError> {
        let rows = sqlx::query_as!(
            TagRow,
            r#"
            SELECT id, slug, name, description, pinned, created_at, updated_at
            FROM tags
            ORDER BY pinned DESC, LOWER(name), slug
            "#
        )
        .fetch_all(self.pool())
        .await
        .map_err(RepoError::from_persistence)?;

        Ok(rows.into_iter().map(TagRecord::from).collect())
    }

    async fn list_for_post(&self, post_id: Uuid) -> Result<Vec<TagRecord>, RepoError> {
        let rows = sqlx::query_as!(
            TagRow,
            r#"
            SELECT t.id, t.slug, t.name, t.description, t.pinned, t.created_at, t.updated_at
            FROM tags t
            INNER JOIN post_tags pt ON pt.tag_id = t.id
            WHERE pt.post_id = $1
            ORDER BY t.name ASC
            "#,
            post_id
        )
        .fetch_all(self.pool())
        .await
        .map_err(RepoError::from_persistence)?;

        Ok(rows.into_iter().map(TagRecord::from).collect())
    }

    async fn list_with_counts(&self) -> Result<Vec<TagWithCount>, RepoError> {
        let rows = sqlx::query!(
            r#"
            SELECT
                t.id,
                t.slug,
                t.name,
                t.pinned,
                COUNT(p.id) AS "count!"
            FROM tags t
            LEFT JOIN post_tags pt ON pt.tag_id = t.id
            LEFT JOIN posts p
                ON p.id = pt.post_id
                AND p.status = 'published'
                AND p.published_at IS NOT NULL
            GROUP BY t.id, t.slug, t.name, t.pinned
            ORDER BY t.pinned DESC, LOWER(t.name), t.slug
            "#
        )
        .fetch_all(self.pool())
        .await
        .map_err(RepoError::from_persistence)?;

        Ok(rows
            .into_iter()
            .map(|row| TagWithCount {
                id: row.id,
                slug: row.slug,
                name: row.name,
                pinned: row.pinned,
                count: row.count,
            })
            .collect())
    }

    async fn find_by_id(&self, id: Uuid) -> Result<Option<TagRecord>, RepoError> {
        let row = sqlx::query_as!(
            TagRow,
            r#"
            SELECT id, slug, name, description, pinned, created_at, updated_at
            FROM tags
            WHERE id = $1
            "#,
            id
        )
        .fetch_optional(self.pool())
        .await
        .map_err(RepoError::from_persistence)?;

        Ok(row.map(TagRecord::from))
    }

    async fn find_by_slug(&self, slug: &str) -> Result<Option<TagRecord>, RepoError> {
        let row = sqlx::query_as!(
            TagRow,
            r#"
            SELECT id, slug, name, description, pinned, created_at, updated_at
            FROM tags
            WHERE slug = $1
            "#,
            slug
        )
        .fetch_optional(self.pool())
        .await
        .map_err(RepoError::from_persistence)?;

        Ok(row.map(TagRecord::from))
    }

    async fn count_usage(&self, id: Uuid) -> Result<u64, RepoError> {
        let row = sqlx::query!(
            r#"
            SELECT COUNT(*) as "count!"
            FROM post_tags
            WHERE tag_id = $1
            "#,
            id
        )
        .fetch_one(self.pool())
        .await
        .map_err(RepoError::from_persistence)?;

        Ok(u64::try_from(row.count).unwrap_or(u64::MAX))
    }

    async fn list_admin_tags(
        &self,
        pinned: Option<bool>,
        filter: &TagQueryFilter,
        page: PageRequest<TagCursor>,
    ) -> Result<CursorPage<TagListRecord>, RepoError> {
        let limit = page.limit.clamp(1, 100) as i64;

        let mut qb = QueryBuilder::new(
            "SELECT t.id, t.slug, t.name, t.description, t.pinned, t.created_at, t.updated_at, \
                    ",
        );
        Self::push_tag_primary_time_expr(&mut qb);
        qb.push(
            " AS primary_time, \
                    (SELECT COUNT(*) \
                     FROM post_tags pt \
                     INNER JOIN posts p \
                        ON p.id = pt.post_id \
                        AND p.status = 'published'::post_status \
                        AND p.published_at IS NOT NULL \
                     WHERE pt.tag_id = t.id) AS usage_count \
             FROM tags t \
             WHERE 1=1 ",
        );

        if let Some(pinned_value) = pinned {
            qb.push(" AND t.pinned = ");
            qb.push_bind(pinned_value);
            qb.push(" ");
        }

        Self::apply_admin_tag_filter(&mut qb, filter);

        if let Some(cursor) = page.cursor {
            qb.push(" AND (t.pinned, ");
            Self::push_tag_primary_time_expr(&mut qb);
            qb.push(", t.id) < (");
            qb.push_bind(cursor.pinned());
            qb.push(", ");
            qb.push_bind(cursor.primary_time());
            qb.push(", ");
            qb.push_bind(cursor.id());
            qb.push(") ");
        }

        qb.push(" ORDER BY t.pinned DESC, primary_time DESC, t.id DESC ");
        qb.push(" LIMIT ");
        qb.push_bind(limit + 1);

        let mut rows = qb
            .build_query_as::<TagListRow>()
            .fetch_all(self.pool())
            .await
            .map_err(RepoError::from_persistence)?;

        let has_more = (rows.len() as i64) > limit;
        if has_more {
            rows.pop();
        }

        let next_cursor = if has_more {
            let last_row = rows
                .last()
                .expect("list_admin_tags rows should be non-empty when truncated");
            let cursor = TagCursor::new(last_row.pinned, last_row.primary_time, last_row.id);
            Some(cursor.encode())
        } else {
            None
        };

        let mut records = Vec::with_capacity(rows.len());
        for row in rows {
            let usage_count = Self::convert_count(row.usage_count)?;
            records.push(TagListRecord {
                id: row.id,
                slug: row.slug,
                name: row.name,
                description: row.description,
                pinned: row.pinned,
                usage_count,
                primary_time: row.primary_time,
                updated_at: row.updated_at,
                created_at: row.created_at,
            });
        }

        Ok(CursorPage::new(records, next_cursor))
    }

    async fn count_tags(
        &self,
        pinned: Option<bool>,
        filter: &TagQueryFilter,
    ) -> Result<u64, RepoError> {
        let mut qb = QueryBuilder::new("SELECT COUNT(*) FROM tags t WHERE 1=1 ");

        if let Some(pinned_value) = pinned {
            qb.push(" AND t.pinned = ");
            qb.push_bind(pinned_value);
            qb.push(" ");
        }

        Self::apply_admin_tag_filter(&mut qb, filter);

        let count: i64 = qb
            .build_query_scalar()
            .fetch_one(self.pool())
            .await
            .map_err(RepoError::from_persistence)?;

        Self::convert_count(count)
    }

    async fn month_counts(
        &self,
        pinned: Option<bool>,
        filter: &TagQueryFilter,
    ) -> Result<Vec<crate::domain::posts::MonthCount>, RepoError> {
        #[derive(sqlx::FromRow)]
        struct MonthRow {
            bucket: OffsetDateTime,
            count: i64,
        }

        let mut qb = QueryBuilder::new("SELECT date_trunc('month', ");
        Self::push_tag_primary_time_expr(&mut qb);
        qb.push(") AS bucket, COUNT(*) AS count FROM tags t WHERE 1=1 ");

        if let Some(pinned_value) = pinned {
            qb.push(" AND t.pinned = ");
            qb.push_bind(pinned_value);
            qb.push(" ");
        }

        Self::apply_admin_tag_filter(&mut qb, filter);
        qb.push(" GROUP BY bucket ORDER BY bucket DESC ");

        let rows: Vec<MonthRow> = qb
            .build_query_as::<MonthRow>()
            .fetch_all(self.pool())
            .await
            .map_err(RepoError::from_persistence)?;

        let mut counts = Vec::with_capacity(rows.len());
        for row in rows {
            let date = row.bucket.date();
            let key = crate::domain::posts::month_key_for(date);
            let label = crate::domain::posts::month_label_for(date);
            counts.push(crate::domain::posts::MonthCount {
                key,
                label,
                count: row.count as usize,
            });
        }

        Ok(counts)
    }
}

#[async_trait]
impl TagsWriteRepo for PostgresRepositories {
    async fn create_tag(&self, params: CreateTagParams) -> Result<TagRecord, RepoError> {
        let CreateTagParams {
            slug,
            name,
            description,
            pinned,
        } = params;

        let id = Uuid::new_v4();
        let now = OffsetDateTime::now_utc();

        let row = sqlx::query_as!(
            TagRow,
            r#"
            INSERT INTO tags (
                id, slug, name, description, pinned,
                created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $6)
            RETURNING id, slug, name, description, pinned, created_at, updated_at
            "#,
            id,
            slug,
            name,
            description,
            pinned,
            now
        )
        .fetch_one(self.pool())
        .await
        .map_err(RepoError::from_persistence)?;

        Ok(TagRecord::from(row))
    }

    async fn update_tag(&self, params: UpdateTagParams) -> Result<TagRecord, RepoError> {
        let UpdateTagParams {
            id,
            slug,
            name,
            description,
            pinned,
        } = params;

        let row = sqlx::query_as!(
            TagRow,
            r#"
            UPDATE tags
            SET slug = $2,
                name = $3,
                description = $4,
                pinned = $5,
                updated_at = now()
            WHERE id = $1
            RETURNING id, slug, name, description, pinned, created_at, updated_at
            "#,
            id,
            slug,
            name,
            description,
            pinned
        )
        .fetch_one(self.pool())
        .await
        .map_err(RepoError::from_persistence)?;

        Ok(TagRecord::from(row))
    }

    async fn delete_tag(&self, id: Uuid) -> Result<(), RepoError> {
        sqlx::query!(
            r#"
            DELETE FROM tags
            WHERE id = $1
            "#,
            id
        )
        .execute(self.pool())
        .await
        .map_err(RepoError::from_persistence)?;

        Ok(())
    }
}

impl PostgresRepositories {
    fn push_tag_primary_time_expr<'q>(qb: &mut QueryBuilder<'q, Postgres>) {
        qb.push(TAG_PRIMARY_TIME_EXPR);
    }

    fn apply_admin_tag_filter<'q>(qb: &mut QueryBuilder<'q, Postgres>, filter: &'q TagQueryFilter) {
        if let Some(month) = filter.month.as_ref() {
            qb.push(" AND to_char(");
            Self::push_tag_primary_time_expr(qb);
            qb.push(", 'YYYY-MM') = ");
            qb.push_bind(month);
            qb.push(" ");
        }

        if let Some(search) = filter.search.as_ref() {
            let pattern = format!("%{}%", search);
            qb.push(" AND (");
            qb.push("t.name ILIKE ");
            qb.push_bind(pattern.clone());
            qb.push(" OR t.slug ILIKE ");
            qb.push_bind(pattern.clone());
            qb.push(" OR COALESCE(t.description, '') ILIKE ");
            qb.push_bind(pattern);
            qb.push(")");
        }
    }
}
