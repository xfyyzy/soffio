use async_trait::async_trait;
use futures::{StreamExt, stream::BoxStream};
use sqlx::QueryBuilder;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{
    application::pagination::{CursorPage, PageCursor},
    application::repos::{PageQueryFilter, PagesRepo, RepoError},
    domain::{entities::PageRecord, types::PageStatus},
};

use super::PostgresRepositories;
use super::types::{PAGE_PRIMARY_TIME_EXPR, PageListRow, PageRow};
use crate::infra::db::map_sqlx_error;

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
        filter: &PageQueryFilter,
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
        filter: &PageQueryFilter,
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
        filter: &PageQueryFilter,
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

#[cfg(test)]
mod tests {}
