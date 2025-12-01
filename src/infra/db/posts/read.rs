use async_trait::async_trait;
use futures::{StreamExt, stream::BoxStream};
use sqlx::QueryBuilder;
use time::OffsetDateTime;

use crate::application::pagination::{CursorPage, PageRequest, PaginationError, PostCursor};
use crate::application::repos::{
    PostListScope, PostQueryFilter, PostTagCount, PostsRepo, RepoError,
};
use crate::domain::entities::PostRecord;
use crate::domain::types::PostStatus;

use super::PostgresRepositories;
use super::types::PostRow;
use crate::infra::db::map_sqlx_error;

impl PostgresRepositories {
    pub fn stream_all_posts(&self) -> BoxStream<'_, Result<PostRecord, RepoError>> {
        let stream = sqlx::query_as!(
            PostRow,
            r#"
            SELECT id, slug, title, excerpt, body_markdown,
                   status AS "status: PostStatus", pinned, scheduled_at,
                   published_at, archived_at, summary_markdown, summary_html,
                   created_at, updated_at,
                   CASE
                       WHEN status = 'published'::post_status THEN COALESCE(published_at, updated_at, created_at)
                       ELSE COALESCE(updated_at, created_at)
                   END AS "primary_time!"
            FROM posts
            ORDER BY slug
            "#
        )
        .fetch(self.pool())
        .map(|row| match row {
            Ok(record) => Ok(PostRecord::from(record)),
            Err(err) => Err(RepoError::from_persistence(err)),
        });

        Box::pin(stream)
    }
}

#[async_trait]
impl PostsRepo for PostgresRepositories {
    async fn list_posts(
        &self,
        scope: PostListScope,
        filter: &PostQueryFilter,
        page: PageRequest<PostCursor>,
    ) -> Result<CursorPage<PostRecord>, RepoError> {
        let limit = page.limit.clamp(1, 100) as i64;

        let mut qb = QueryBuilder::new(
            "SELECT p.id, p.slug, p.title, p.excerpt, p.body_markdown, p.status, \
             p.pinned, p.scheduled_at, p.published_at, p.archived_at, p.summary_markdown, \
             p.summary_html, p.created_at, p.updated_at, ",
        );
        Self::push_primary_time_expr(&mut qb);
        qb.push(" AS primary_time FROM posts p WHERE 1=1 ");

        Self::apply_scope_conditions(&mut qb, scope);
        Self::apply_feed_filter(&mut qb, filter);

        if let Some(cursor) = page.cursor {
            match scope {
                PostListScope::Public => {
                    qb.push(" AND (p.pinned, p.published_at, p.id) < (");
                    qb.push_bind(cursor.pinned());
                    qb.push(", ");
                    qb.push_bind(cursor.sort_key());
                    qb.push(", ");
                    qb.push_bind(cursor.id());
                    qb.push(")");
                }
                PostListScope::Admin { .. } => {
                    cursor.status().ok_or_else(|| {
                        RepoError::Pagination(PaginationError::InvalidCursor(
                            "cursor missing status for admin scope".to_string(),
                        ))
                    })?;
                    qb.push(" AND (p.pinned, ");
                    Self::push_primary_time_expr(&mut qb);
                    qb.push(", p.id) < (");
                    qb.push_bind(cursor.pinned());
                    qb.push(", ");
                    qb.push_bind(cursor.sort_key());
                    qb.push(", ");
                    qb.push_bind(cursor.id());
                    qb.push(")");
                }
            }
        }

        match scope {
            PostListScope::Public => {
                qb.push(" ORDER BY p.pinned DESC, p.published_at DESC NULLS LAST, p.id DESC ");
            }
            PostListScope::Admin { .. } => {
                qb.push(" ORDER BY p.pinned DESC, primary_time DESC, p.id DESC ");
            }
        }

        qb.push(" LIMIT ");
        qb.push_bind(limit + 1);

        let mut rows = qb
            .build_query_as::<PostRow>()
            .fetch_all(self.pool())
            .await
            .map_err(map_sqlx_error)?;

        let mut records = Vec::with_capacity(rows.len());
        let has_more = (rows.len() as i64) > limit;
        if has_more {
            rows.pop();
        }

        let next_cursor = if has_more {
            let last_row = rows
                .last()
                .expect("page should contain at least one row when truncated");
            let sort_key = last_row.primary_time;
            let cursor = match scope {
                PostListScope::Public => {
                    PostCursor::published(sort_key, last_row.id, last_row.pinned)
                }
                PostListScope::Admin { status } => {
                    let status = status.unwrap_or(last_row.status);
                    PostCursor::admin(status, sort_key, last_row.id, last_row.pinned)
                }
            };
            Some(cursor.encode())
        } else {
            None
        };

        for row in rows {
            records.push(PostRecord::from(row));
        }

        Ok(CursorPage::new(records, next_cursor))
    }

    async fn count_posts(
        &self,
        scope: PostListScope,
        filter: &PostQueryFilter,
    ) -> Result<u64, RepoError> {
        let mut qb = QueryBuilder::new("SELECT COUNT(*) FROM posts p WHERE 1=1 ");
        Self::apply_scope_conditions(&mut qb, scope);
        Self::apply_feed_filter(&mut qb, filter);

        let count: i64 = qb
            .build_query_scalar()
            .fetch_one(self.pool())
            .await
            .map_err(map_sqlx_error)?;

        Self::convert_count(count)
    }

    async fn count_posts_before(
        &self,
        scope: PostListScope,
        filter: &PostQueryFilter,
        cursor: &PostCursor,
    ) -> Result<u64, RepoError> {
        let mut qb = QueryBuilder::new("SELECT COUNT(*) FROM posts p WHERE 1=1 ");
        Self::apply_scope_conditions(&mut qb, scope);
        Self::apply_feed_filter(&mut qb, filter);

        match scope {
            PostListScope::Public => {
                qb.push(" AND (p.pinned, p.published_at, p.id) >= (");
                qb.push_bind(cursor.pinned());
                qb.push(", ");
                qb.push_bind(cursor.sort_key());
                qb.push(", ");
                qb.push_bind(cursor.id());
                qb.push(")");
            }
            PostListScope::Admin { .. } => {
                cursor.status().ok_or_else(|| {
                    RepoError::Pagination(PaginationError::InvalidCursor(
                        "cursor missing status for admin scope".to_string(),
                    ))
                })?;
                qb.push(" AND (p.pinned, ");
                Self::push_primary_time_expr(&mut qb);
                qb.push(", p.id) >= (");
                qb.push_bind(cursor.pinned());
                qb.push(", ");
                qb.push_bind(cursor.sort_key());
                qb.push(", ");
                qb.push_bind(cursor.id());
                qb.push(")");
            }
        }

        let count: i64 = qb
            .build_query_scalar()
            .fetch_one(self.pool())
            .await
            .map_err(map_sqlx_error)?;

        Self::convert_count(count)
    }

    async fn list_month_counts(
        &self,
        scope: PostListScope,
        filter: &PostQueryFilter,
    ) -> Result<Vec<crate::domain::posts::MonthCount>, RepoError> {
        #[derive(sqlx::FromRow)]
        struct MonthRow {
            bucket: OffsetDateTime,
            count: i64,
        }

        let mut qb = QueryBuilder::new("SELECT date_trunc('month', ");
        Self::push_primary_time_expr(&mut qb);
        qb.push(") AS bucket, COUNT(*) AS count FROM posts p WHERE 1=1 ");
        Self::apply_scope_conditions(&mut qb, scope);
        Self::apply_feed_filter(&mut qb, filter);

        qb.push(" AND ");
        Self::push_primary_time_expr(&mut qb);
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

    async fn list_tag_counts(
        &self,
        scope: PostListScope,
        filter: &PostQueryFilter,
    ) -> Result<Vec<PostTagCount>, RepoError> {
        #[derive(sqlx::FromRow)]
        struct TagRow {
            slug: String,
            name: String,
            count: i64,
        }

        let mut qb = QueryBuilder::new(
            "SELECT t.slug, t.name, COUNT(*) AS count \
             FROM posts p \
             INNER JOIN post_tags pt ON pt.post_id = p.id \
             INNER JOIN tags t ON t.id = pt.tag_id \
             WHERE 1=1 ",
        );

        Self::apply_scope_conditions(&mut qb, scope);

        let mut effective_filter = filter.clone();
        effective_filter.tag = None;
        Self::apply_feed_filter(&mut qb, &effective_filter);

        qb.push(" GROUP BY t.slug, t.name ");
        qb.push(" ORDER BY count DESC, LOWER(t.name), t.slug ");

        let rows: Vec<TagRow> = qb
            .build_query_as::<TagRow>()
            .fetch_all(self.pool())
            .await
            .map_err(map_sqlx_error)?;

        let mut counts = Vec::with_capacity(rows.len());
        for row in rows {
            let count = Self::convert_count(row.count)?;
            counts.push(PostTagCount {
                slug: row.slug,
                name: row.name,
                count,
            });
        }

        Ok(counts)
    }

    async fn find_by_slug(&self, slug: &str) -> Result<Option<PostRecord>, RepoError> {
        let row = sqlx::query_as!(
            PostRow,
            r#"
            SELECT id, slug, title, excerpt, body_markdown,
                   status AS "status: PostStatus", pinned, scheduled_at,
                   published_at, archived_at, summary_markdown, summary_html,
                    created_at, updated_at,
                   CASE
                       WHEN status = 'published'::post_status THEN COALESCE(published_at, updated_at, created_at)
                       ELSE COALESCE(updated_at, created_at)
                   END AS "primary_time!"
            FROM posts
            WHERE slug = $1
            "#,
            slug
        )
        .fetch_optional(self.pool())
        .await
        .map_err(map_sqlx_error)?;

        Ok(row.map(PostRecord::from))
    }

    async fn find_by_id(&self, id: uuid::Uuid) -> Result<Option<PostRecord>, RepoError> {
        let row = sqlx::query_as!(
            PostRow,
            r#"
            SELECT id, slug, title, excerpt, body_markdown,
                   status AS "status: PostStatus", pinned, scheduled_at,
                   published_at, archived_at, summary_markdown, summary_html,
                   created_at, updated_at,
                   CASE
                       WHEN status = 'published'::post_status THEN COALESCE(published_at, updated_at, created_at)
                       ELSE COALESCE(updated_at, created_at)
                   END AS "primary_time!"
            FROM posts
            WHERE id = $1
            "#,
            id
        )
        .fetch_optional(self.pool())
        .await
        .map_err(map_sqlx_error)?;

        Ok(row.map(PostRecord::from))
    }
}

#[cfg(test)]
mod tests {}
