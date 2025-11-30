use async_trait::async_trait;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::application::repos::{
    CreatePostParams, PostsWriteRepo, RepoError, UpdatePostParams, UpdatePostPinnedParams,
    UpdatePostStatusParams,
};
use crate::domain::entities::PostRecord;
use crate::domain::types::PostStatus;

use super::PostgresRepositories;
use super::types::PostRow;

fn map_sqlx_error(err: sqlx::Error) -> RepoError {
    match err {
        sqlx::Error::RowNotFound => RepoError::NotFound,
        sqlx::Error::Database(db) if db.message().contains("duplicate key") => {
            RepoError::Duplicate {
                constraint: db.constraint().unwrap_or("unknown").to_string(),
            }
        }
        sqlx::Error::Database(db)
            if db.message().contains("violates foreign key constraint")
                || db.message().contains("invalid input syntax") =>
        {
            RepoError::InvalidInput {
                message: db.message().to_string(),
            }
        }
        sqlx::Error::Database(db) if db.message().contains("violates") => RepoError::Integrity {
            message: db.message().to_string(),
        },
        sqlx::Error::Database(db)
            if db
                .message()
                .contains("canceling statement due to user request") =>
        {
            RepoError::Timeout
        }
        other => RepoError::from_persistence(other),
    }
}

#[async_trait]
impl PostsWriteRepo for PostgresRepositories {
    async fn create_post(&self, params: CreatePostParams) -> Result<PostRecord, RepoError> {
        let CreatePostParams {
            slug,
            title,
            excerpt,
            body_markdown,
            status,
            pinned,
            scheduled_at,
            published_at,
            archived_at,
            summary_markdown,
            summary_html,
        } = params;

        let id = Uuid::new_v4();
        let now = OffsetDateTime::now_utc();
        let row = sqlx::query_as!(
            PostRow,
            r#"
            INSERT INTO posts (
                id, slug, title, excerpt, body_markdown, status, pinned,
                scheduled_at, published_at, archived_at, summary_markdown, summary_html,
                created_at, updated_at
            )
            VALUES (
                $1, $2, $3, $4, $5, $6, $7,
                $8, $9, $10, $11, $12,
                $13, $13
            )
            RETURNING id, slug, title, excerpt, body_markdown,
                     status AS "status: PostStatus", pinned, scheduled_at, published_at, archived_at,
                     summary_markdown, summary_html, created_at, updated_at,
                     CASE
                         WHEN status = 'published'::post_status THEN COALESCE(published_at, updated_at, created_at)
                         ELSE COALESCE(updated_at, created_at)
                     END AS "primary_time!"
            "#,
            id,
            slug,
            title,
            excerpt,
            body_markdown,
            status as PostStatus,
            pinned,
            scheduled_at,
            published_at,
            archived_at,
            summary_markdown,
            summary_html,
            now
        )
        .fetch_one(self.pool())
        .await
        .map_err(map_sqlx_error)?;

        Ok(PostRecord::from(row))
    }

    async fn update_post(&self, params: UpdatePostParams) -> Result<PostRecord, RepoError> {
        let UpdatePostParams {
            id,
            slug,
            title,
            excerpt,
            body_markdown,
            pinned,
            summary_markdown,
            summary_html,
        } = params;

        let now = OffsetDateTime::now_utc();
        let row = sqlx::query_as!(
            PostRow,
            r#"
            UPDATE posts
            SET slug = $2,
                title = $3,
                excerpt = $4,
                body_markdown = $5,
                pinned = $6,
                summary_markdown = $7,
                summary_html = $8,
                updated_at = $9
            WHERE id = $1
            RETURNING id, slug, title, excerpt, body_markdown,
                     status AS "status: PostStatus", pinned, scheduled_at, published_at, archived_at,
                     summary_markdown, summary_html, created_at, updated_at,
                     CASE
                         WHEN status = 'published'::post_status THEN COALESCE(published_at, updated_at, created_at)
                         ELSE COALESCE(updated_at, created_at)
                     END AS "primary_time!"
            "#,
            id,
            slug,
            title,
            excerpt,
            body_markdown,
            pinned,
            summary_markdown,
            summary_html,
            now
        )
        .fetch_one(self.pool())
        .await
        .map_err(map_sqlx_error)?;

        Ok(PostRecord::from(row))
    }

    async fn update_post_status(
        &self,
        params: UpdatePostStatusParams,
    ) -> Result<PostRecord, RepoError> {
        let UpdatePostStatusParams {
            id,
            status,
            scheduled_at,
            published_at,
            archived_at,
        } = params;

        let now = OffsetDateTime::now_utc();

        let row = sqlx::query_as!(
            PostRow,
            r#"
            UPDATE posts
            SET status = $2,
                scheduled_at = $3,
                published_at = $4,
                archived_at = $5,
                updated_at = $6
            WHERE id = $1
            RETURNING id, slug, title, excerpt, body_markdown,
                     status AS "status: PostStatus", pinned, scheduled_at, published_at, archived_at,
                     summary_markdown, summary_html, created_at, updated_at,
                     CASE
                         WHEN status = 'published'::post_status THEN COALESCE(published_at, updated_at, created_at)
                         ELSE COALESCE(updated_at, created_at)
                     END AS "primary_time!"
            "#,
            id,
            status as PostStatus,
            scheduled_at,
            published_at,
            archived_at,
            now
        )
        .fetch_one(self.pool())
        .await
        .map_err(map_sqlx_error)?;

        Ok(PostRecord::from(row))
    }

    async fn update_post_pinned(
        &self,
        params: UpdatePostPinnedParams,
    ) -> Result<PostRecord, RepoError> {
        let UpdatePostPinnedParams { id, pinned } = params;

        let row = sqlx::query_as!(
            PostRow,
            r#"
            UPDATE posts
               SET pinned = $2,
                   updated_at = now()
             WHERE id = $1
            RETURNING id, slug, title, excerpt, body_markdown,
                     status AS "status: PostStatus", pinned, scheduled_at, published_at, archived_at,
                     summary_markdown, summary_html, created_at, updated_at,
                     CASE
                         WHEN status = 'published'::post_status THEN COALESCE(published_at, updated_at, created_at)
                         ELSE COALESCE(updated_at, created_at)
                     END AS "primary_time!"
            "#,
            id,
            pinned
        )
        .fetch_one(self.pool())
        .await
        .map_err(map_sqlx_error)?;

        Ok(PostRecord::from(row))
    }

    async fn schedule_post_publication(
        &self,
        id: Uuid,
        publish_at: OffsetDateTime,
    ) -> Result<PostRecord, RepoError> {
        let row = sqlx::query_as!(
            PostRow,
            r#"
            UPDATE posts
               SET scheduled_at = $2,
                   published_at = NULL,
                   status = $3,
                   updated_at = now()
             WHERE id = $1
            RETURNING id, slug, title, excerpt, body_markdown,
                     status AS "status: PostStatus", pinned, scheduled_at, published_at, archived_at,
                     summary_markdown, summary_html, created_at, updated_at,
                     CASE
                         WHEN status = 'published'::post_status THEN COALESCE(published_at, updated_at, created_at)
                         ELSE COALESCE(updated_at, created_at)
                     END AS "primary_time!"
            "#,
            id,
            publish_at,
            PostStatus::Draft as PostStatus
        )
        .fetch_one(self.pool())
        .await
        .map_err(map_sqlx_error)?;

        Ok(PostRecord::from(row))
    }

    async fn delete_post(&self, id: Uuid) -> Result<(), RepoError> {
        sqlx::query!(
            r#"
            DELETE FROM posts
            WHERE id = $1
            "#,
            id
        )
        .execute(self.pool())
        .await
        .map_err(map_sqlx_error)?;

        Ok(())
    }

    async fn replace_post_tags(&self, post_id: Uuid, tag_ids: &[Uuid]) -> Result<(), RepoError> {
        let mut tx = self.pool().begin().await.map_err(map_sqlx_error)?;

        sqlx::query!(
            r#"
            DELETE FROM post_tags
            WHERE post_id = $1
            "#,
            post_id
        )
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx_error)?;

        if !tag_ids.is_empty() {
            sqlx::query!(
                r#"
                INSERT INTO post_tags (post_id, tag_id)
                SELECT $1, id
                FROM UNNEST($2::uuid[]) AS id
                "#,
                post_id,
                tag_ids
            )
            .execute(&mut *tx)
            .await
            .map_err(map_sqlx_error)?;
        }

        tx.commit().await.map_err(map_sqlx_error)?;

        Ok(())
    }
}
