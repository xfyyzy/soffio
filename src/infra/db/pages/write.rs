use async_trait::async_trait;
use sqlx::{Postgres, Transaction};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{
    application::repos::{
        CreatePageParams, PagesWriteRepo, RepoError, RestorePageSnapshotParams, UpdatePageParams,
        UpdatePageStatusParams,
    },
    domain::{entities::PageRecord, types::PageStatus},
};

use super::PostgresRepositories;
use super::types::PageRow;
use crate::infra::db::map_sqlx_error;

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

    async fn restore_page_snapshot(
        &self,
        params: RestorePageSnapshotParams,
    ) -> Result<PageRecord, RepoError> {
        let now = OffsetDateTime::now_utc();
        let RestorePageSnapshotParams {
            id,
            slug,
            title,
            body_markdown,
            rendered_html,
            status,
            scheduled_at,
            published_at,
            archived_at,
        } = params;

        let row = sqlx::query_as!(
            PageRow,
            r#"
            UPDATE pages
            SET slug = $2,
                title = $3,
                body_markdown = $4,
                rendered_html = $5,
                status = $6,
                scheduled_at = $7,
                published_at = $8,
                archived_at = $9,
                updated_at = $10
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
