use async_trait::async_trait;
use futures::{StreamExt, stream::BoxStream};
use sqlx::{Postgres, QueryBuilder, Transaction};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{
    application::pagination::{CursorPage, PageCursor},
    application::repos::{
        CreatePageParams, PagesRepo, PagesWriteRepo, RepoError, UpdatePageParams,
        UpdatePageStatusParams,
    },
    domain::{entities::PageRecord, types::PageStatus},
};

use super::{PostgresRepositories, map_sqlx_error};

const PAGE_PRIMARY_TIME_EXPR: &str = "CASE \
    WHEN status = 'published'::page_status THEN \
        COALESCE(published_at, updated_at, created_at) \
    ELSE \
        COALESCE(updated_at, created_at) \
END";

#[derive(sqlx::FromRow)]
struct PageRow {
    id: Uuid,
    slug: String,
    title: String,
    body_markdown: String,
    rendered_html: String,
    status: PageStatus,
    scheduled_at: Option<OffsetDateTime>,
    published_at: Option<OffsetDateTime>,
    archived_at: Option<OffsetDateTime>,
    created_at: OffsetDateTime,
    updated_at: OffsetDateTime,
}

impl From<PageRow> for PageRecord {
    fn from(row: PageRow) -> Self {
        Self {
            id: row.id,
            slug: row.slug,
            title: row.title,
            body_markdown: row.body_markdown,
            rendered_html: row.rendered_html,
            status: row.status,
            scheduled_at: row.scheduled_at,
            published_at: row.published_at,
            archived_at: row.archived_at,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

#[derive(sqlx::FromRow)]
struct PageListRow {
    id: Uuid,
    slug: String,
    title: String,
    body_markdown: String,
    rendered_html: String,
    status: PageStatus,
    scheduled_at: Option<OffsetDateTime>,
    published_at: Option<OffsetDateTime>,
    archived_at: Option<OffsetDateTime>,
    created_at: OffsetDateTime,
    updated_at: OffsetDateTime,
    primary_time: OffsetDateTime,
}

impl From<PageListRow> for PageRecord {
    fn from(row: PageListRow) -> Self {
        Self {
            id: row.id,
            slug: row.slug,
            title: row.title,
            body_markdown: row.body_markdown,
            rendered_html: row.rendered_html,
            status: row.status,
            scheduled_at: row.scheduled_at,
            published_at: row.published_at,
            archived_at: row.archived_at,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

impl PostgresRepositories {
    pub fn stream_all_pages(&self) -> BoxStream<'_, Result<PageRecord, RepoError>> {
        let stream = sqlx::query_as!(
            PageRow,
            r#"
            SELECT id, slug, title, body_markdown, rendered_html,
                   status AS "status: PageStatus",
                   scheduled_at, published_at, archived_at,
                   created_at, updated_at
            FROM pages
            ORDER BY slug
            "#
        )
        .fetch(self.pool())
        .map(|row| match row {
            Ok(record) => Ok(PageRecord::from(record)),
            Err(err) => Err(RepoError::from_persistence(err)),
        });

        Box::pin(stream)
    }
}

#[async_trait]
impl PagesRepo for PostgresRepositories {
    async fn list_pages(
        &self,
        status: Option<PageStatus>,
        limit: u32,
        cursor: Option<PageCursor>,
        filter: &crate::application::repos::PageQueryFilter,
    ) -> Result<CursorPage<PageRecord>, RepoError> {
        let limit = limit.clamp(1, 100) as i64;
        let mut qb = QueryBuilder::new(
            "SELECT id, slug, title, body_markdown, rendered_html, status, \
             scheduled_at, published_at, archived_at, created_at, updated_at, ",
        );
        qb.push(PAGE_PRIMARY_TIME_EXPR);
        qb.push(" AS primary_time FROM pages WHERE 1=1 ");

        if let Some(status) = status {
            qb.push("AND status = ");
            qb.push_bind(status);
        }

        if let Some(search) = filter.search.as_ref() {
            qb.push(" AND (");
            qb.push("title ILIKE ");
            qb.push_bind(format!("%{}%", search));
            qb.push(" OR slug ILIKE ");
            qb.push_bind(format!("%{}%", search));
            qb.push(" OR rendered_html ILIKE ");
            qb.push_bind(format!("%{}%", search));
            qb.push(")");
        }

        if let Some(month) = filter.month.as_ref() {
            qb.push(" AND to_char(");
            qb.push(PAGE_PRIMARY_TIME_EXPR);
            qb.push(", 'YYYY-MM') = ");
            qb.push_bind(month);
            qb.push(" ");
        }

        if let Some(month) = filter.month.as_ref() {
            qb.push(" AND to_char(");
            qb.push(PAGE_PRIMARY_TIME_EXPR);
            qb.push(", 'YYYY-MM') = ");
            qb.push_bind(month);
            qb.push(" ");
        }

        if let Some(cursor) = cursor {
            qb.push(" AND (");
            qb.push(PAGE_PRIMARY_TIME_EXPR);
            qb.push(", id) < (");
            qb.push_bind(cursor.primary_time());
            qb.push(", ");
            qb.push_bind(cursor.id());
            qb.push(")");
        }

        qb.push(" ORDER BY ");
        qb.push(PAGE_PRIMARY_TIME_EXPR);
        qb.push(" DESC, id DESC");
        qb.push(" LIMIT ");
        qb.push_bind(limit + 1);

        let mut rows = qb
            .build_query_as::<PageListRow>()
            .fetch_all(self.pool())
            .await
            .map_err(map_sqlx_error)?;

        let next_cursor = if (rows.len() as i64) > limit {
            let overflow = rows.pop().expect("overflow row exists when len > limit");
            Some(PageCursor::new(overflow.primary_time, overflow.id).encode())
        } else {
            None
        };

        let records = rows.into_iter().map(PageRecord::from).collect();

        Ok(CursorPage::new(records, next_cursor))
    }

    async fn find_by_slug(&self, slug: &str) -> Result<Option<PageRecord>, RepoError> {
        let row = sqlx::query_as!(
            PageRow,
            r#"
            SELECT id, slug, title, body_markdown, rendered_html,
                   status AS "status: PageStatus",
                   scheduled_at, published_at, archived_at, created_at, updated_at
            FROM pages
            WHERE slug = $1
            "#,
            slug
        )
        .fetch_optional(self.pool())
        .await
        .map_err(map_sqlx_error)?;

        Ok(row.map(PageRecord::from))
    }

    async fn count_pages(
        &self,
        status: Option<PageStatus>,
        filter: &crate::application::repos::PageQueryFilter,
    ) -> Result<u64, RepoError> {
        let mut qb = QueryBuilder::new("SELECT COUNT(*) FROM pages WHERE 1=1 ");

        if let Some(status) = status {
            qb.push("AND status = ");
            qb.push_bind(status);
        }

        if let Some(search) = filter.search.as_ref() {
            qb.push(" AND (");
            qb.push("title ILIKE ");
            qb.push_bind(format!("%{}%", search));
            qb.push(" OR slug ILIKE ");
            qb.push_bind(format!("%{}%", search));
            qb.push(" OR rendered_html ILIKE ");
            qb.push_bind(format!("%{}%", search));
            qb.push(")");
        }

        if let Some(month) = filter.month.as_ref() {
            qb.push(" AND to_char(");
            qb.push(PAGE_PRIMARY_TIME_EXPR);
            qb.push(", 'YYYY-MM') = ");
            qb.push_bind(month);
            qb.push(" ");
        }

        let count: i64 = qb
            .build_query_scalar()
            .fetch_one(self.pool())
            .await
            .map_err(map_sqlx_error)?;

        Ok(count as u64)
    }

    async fn find_by_id(&self, id: Uuid) -> Result<Option<PageRecord>, RepoError> {
        let row = sqlx::query_as!(
            PageRow,
            r#"
            SELECT id, slug, title, body_markdown, rendered_html,
                   status AS "status: PageStatus",
                   scheduled_at, published_at, archived_at, created_at, updated_at
            FROM pages
            WHERE id = $1
            "#,
            id
        )
        .fetch_optional(self.pool())
        .await
        .map_err(map_sqlx_error)?;

        Ok(row.map(PageRecord::from))
    }

    async fn list_month_counts(
        &self,
        status: Option<PageStatus>,
        filter: &crate::application::repos::PageQueryFilter,
    ) -> Result<Vec<crate::domain::posts::MonthCount>, RepoError> {
        #[derive(sqlx::FromRow)]
        struct MonthRow {
            bucket: OffsetDateTime,
            count: i64,
        }

        let mut qb = QueryBuilder::new("SELECT date_trunc('month', ");
        qb.push(PAGE_PRIMARY_TIME_EXPR);
        qb.push(") AS bucket, COUNT(*) AS count FROM pages WHERE 1=1 ");

        if let Some(status) = status {
            qb.push("AND status = ");
            qb.push_bind(status);
            qb.push(" ");
        }

        if let Some(search) = filter.search.as_ref() {
            qb.push(" AND (");
            qb.push("title ILIKE ");
            qb.push_bind(format!("%{}%", search));
            qb.push(" OR slug ILIKE ");
            qb.push_bind(format!("%{}%", search));
            qb.push(" OR rendered_html ILIKE ");
            qb.push_bind(format!("%{}%", search));
            qb.push(")");
        }

        qb.push(" AND ");
        qb.push(PAGE_PRIMARY_TIME_EXPR);
        qb.push(" IS NOT NULL ");
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
impl PagesWriteRepo for PostgresRepositories {
    async fn create_page(&self, params: CreatePageParams) -> Result<PageRecord, RepoError> {
        let CreatePageParams {
            slug,
            title,
            body_markdown,
            rendered_html,
            status,
            scheduled_at,
            published_at,
            archived_at,
        } = params;

        let id = Uuid::new_v4();
        let now = OffsetDateTime::now_utc();

        let row = sqlx::query_as!(
            PageRow,
            r#"
            INSERT INTO pages (
                id, slug, title, body_markdown, rendered_html, status,
                scheduled_at, published_at, archived_at,
                created_at, updated_at
            )
            VALUES (
                $1, $2, $3, $4, $5, $6,
                $7, $8, $9,
                $10, $10
            )
            RETURNING id, slug, title, body_markdown, rendered_html,
                     status AS "status: PageStatus",
                     scheduled_at, published_at, archived_at, created_at, updated_at
            "#,
            id,
            slug,
            title,
            body_markdown,
            rendered_html,
            status as PageStatus,
            scheduled_at,
            published_at,
            archived_at,
            now
        )
        .fetch_one(self.pool())
        .await
        .map_err(map_sqlx_error)?;

        Ok(PageRecord::from(row))
    }

    async fn update_page(&self, params: UpdatePageParams) -> Result<PageRecord, RepoError> {
        let UpdatePageParams {
            id,
            slug,
            title,
            body_markdown,
            rendered_html,
        } = params;

        let now = OffsetDateTime::now_utc();

        let row = sqlx::query_as!(
            PageRow,
            r#"
            UPDATE pages
            SET slug = $2,
                title = $3,
                body_markdown = $4,
                rendered_html = $5,
                updated_at = $6
            WHERE id = $1
            RETURNING id, slug, title, body_markdown, rendered_html,
                     status AS "status: PageStatus",
                     scheduled_at, published_at, archived_at, created_at, updated_at
            "#,
            id,
            slug,
            title,
            body_markdown,
            rendered_html,
            now
        )
        .fetch_one(self.pool())
        .await
        .map_err(map_sqlx_error)?;

        Ok(PageRecord::from(row))
    }

    async fn update_page_status(
        &self,
        params: UpdatePageStatusParams,
    ) -> Result<PageRecord, RepoError> {
        let UpdatePageStatusParams {
            id,
            status,
            scheduled_at,
            published_at,
            archived_at,
        } = params;

        let now = OffsetDateTime::now_utc();

        let row = sqlx::query_as!(
            PageRow,
            r#"
            UPDATE pages
            SET status = $2,
                scheduled_at = $3,
                published_at = $4,
                archived_at = $5,
                updated_at = $6
            WHERE id = $1
            RETURNING id, slug, title, body_markdown, rendered_html,
                     status AS "status: PageStatus",
                     scheduled_at, published_at, archived_at, created_at, updated_at
            "#,
            id,
            status as PageStatus,
            scheduled_at,
            published_at,
            archived_at,
            now
        )
        .fetch_one(self.pool())
        .await
        .map_err(map_sqlx_error)?;

        Ok(PageRecord::from(row))
    }

    async fn schedule_page_publication(
        &self,
        id: Uuid,
        publish_at: OffsetDateTime,
    ) -> Result<PageRecord, RepoError> {
        let row = sqlx::query_as!(
            PageRow,
            r#"
            UPDATE pages
               SET scheduled_at = $2,
                   published_at = NULL,
                   status = $3,
                   updated_at = now()
             WHERE id = $1
            RETURNING id, slug, title, body_markdown, rendered_html,
                     status AS "status: PageStatus",
                     scheduled_at, published_at, archived_at, created_at, updated_at
            "#,
            id,
            publish_at,
            PageStatus::Draft as PageStatus
        )
        .fetch_one(self.pool())
        .await
        .map_err(map_sqlx_error)?;

        Ok(PageRecord::from(row))
    }

    async fn delete_page(&self, id: Uuid) -> Result<(), RepoError> {
        sqlx::query!(
            r#"
            DELETE FROM pages
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

impl PostgresRepositories {
    pub async fn find_page_id_by_slug_immediate(
        &self,
        slug: &str,
    ) -> Result<Option<Uuid>, RepoError> {
        let row = sqlx::query!(
            r#"
            SELECT id
            FROM pages
            WHERE slug = $1
            "#,
            slug
        )
        .fetch_optional(self.pool())
        .await
        .map_err(map_sqlx_error)?;

        Ok(row.map(|record| record.id))
    }

    pub async fn find_page_id_by_slug(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        slug: &str,
    ) -> Result<Option<Uuid>, RepoError> {
        let row = sqlx::query!(
            r#"
            SELECT id
            FROM pages
            WHERE slug = $1
            "#,
            slug
        )
        .fetch_optional(tx.as_mut())
        .await
        .map_err(map_sqlx_error)?;

        Ok(row.map(|record| record.id))
    }

    pub async fn update_page_rendered_html(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        page_id: Uuid,
        rendered_html: &str,
    ) -> Result<(), RepoError> {
        sqlx::query!(
            r#"
            UPDATE pages
            SET rendered_html = $2,
                updated_at = now()
            WHERE id = $1
            "#,
            page_id,
            rendered_html
        )
        .execute(tx.as_mut())
        .await
        .map_err(map_sqlx_error)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {}
