use std::sync::Arc;

use apalis::prelude::{Data, Error as ApalisError};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use tracing::{error, info};
use uuid::Uuid;

use crate::{
    application::repos::{AuditRepo, JobsRepo, RepoError},
    domain::entities::AuditLogRecord,
    domain::types::JobType,
    infra::db::PostgresRepositories,
};

use super::{
    context::{JobWorkerContext, job_failed},
    queue::enqueue_job,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublishPostJobPayload {
    pub slug: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublishPageJobPayload {
    pub slug: String,
}

pub async fn enqueue_publish_post_job<J: JobsRepo + ?Sized>(
    repo: &J,
    slug: String,
    run_at: OffsetDateTime,
) -> Result<String, RepoError> {
    let payload = PublishPostJobPayload { slug };
    enqueue_job(repo, JobType::PublishPost, &payload, Some(run_at), 10, 10).await
}

pub async fn enqueue_publish_page_job<J: JobsRepo + ?Sized>(
    repo: &J,
    slug: String,
    run_at: OffsetDateTime,
) -> Result<String, RepoError> {
    let payload = PublishPageJobPayload { slug };
    enqueue_job(repo, JobType::PublishPage, &payload, Some(run_at), 10, 10).await
}

pub async fn process_publish_post_job(
    payload: PublishPostJobPayload,
    context: Data<JobWorkerContext>,
) -> Result<(), ApalisError> {
    let ctx = &*context;
    let repositories = ctx.repositories.clone();

    let post_row = sqlx::query!(
        r#"
        SELECT id, scheduled_at
        FROM posts
        WHERE slug = $1
        "#,
        payload.slug
    )
    .fetch_optional(repositories.pool())
    .await
    .map_err(job_failed)?;

    let Some(post_row) = post_row else {
        return Err(job_failed(RepoError::from_persistence(format!(
            "post `{}` not found",
            payload.slug
        ))));
    };

    let publish_at = post_row
        .scheduled_at
        .unwrap_or_else(OffsetDateTime::now_utc);

    let entity_id = mark_post_published(
        Arc::clone(&repositories),
        post_row.id,
        publish_at,
        &payload.slug,
    )
    .await?;

    record_audit_event(
        Arc::clone(&repositories),
        "post.publish",
        "post",
        entity_id,
        payload.slug.clone(),
    )
    .await;

    info!(
        target = "application::jobs::process_publish_post_job",
        slug = payload.slug,
        "post published"
    );

    Ok(())
}

pub async fn process_publish_page_job(
    payload: PublishPageJobPayload,
    context: Data<JobWorkerContext>,
) -> Result<(), ApalisError> {
    let ctx = &*context;
    let repositories = ctx.repositories.clone();

    let page_row = sqlx::query!(
        r#"
        SELECT id, scheduled_at
        FROM pages
        WHERE slug = $1
        "#,
        payload.slug
    )
    .fetch_optional(repositories.pool())
    .await
    .map_err(job_failed)?;

    let Some(page_row) = page_row else {
        return Err(job_failed(RepoError::from_persistence(format!(
            "page `{}` not found",
            payload.slug
        ))));
    };

    let publish_at = page_row
        .scheduled_at
        .unwrap_or_else(OffsetDateTime::now_utc);

    let entity_id = mark_page_published(
        Arc::clone(&repositories),
        page_row.id,
        publish_at,
        &payload.slug,
    )
    .await?;

    record_audit_event(
        Arc::clone(&repositories),
        "page.publish",
        "page",
        entity_id,
        payload.slug.clone(),
    )
    .await;

    info!(
        target = "application::jobs::process_publish_page_job",
        slug = payload.slug,
        "page published"
    );

    Ok(())
}

async fn record_audit_event(
    repositories: Arc<PostgresRepositories>,
    action: &'static str,
    entity_type: &'static str,
    entity_id: Uuid,
    slug: String,
) {
    let audit_record = AuditLogRecord {
        id: Uuid::new_v4(),
        actor: "system".to_string(),
        action: action.to_string(),
        entity_type: entity_type.to_string(),
        entity_id: Some(entity_id.to_string()),
        payload_text: Some(slug),
        created_at: OffsetDateTime::now_utc(),
    };

    if let Err(err) = repositories.append_log(audit_record).await {
        error!(
            target = "application::jobs::publish",
            error = %err,
            "failed to append audit log"
        );
    }
}

async fn mark_post_published(
    repositories: Arc<PostgresRepositories>,
    post_id: Uuid,
    publish_at: OffsetDateTime,
    slug: &str,
) -> Result<Uuid, ApalisError> {
    let mut tx = repositories.begin().await.map_err(job_failed)?;

    repositories
        .update_post_updated_at(&mut tx, post_id)
        .await
        .map_err(job_failed)?;

    let updated = sqlx::query!(
        r#"
        UPDATE posts
           SET status = 'published'::post_status,
               published_at = $2,
               scheduled_at = NULL,
               updated_at = now()
         WHERE id = $1
        RETURNING id
        "#,
        post_id,
        publish_at
    )
    .fetch_optional(tx.as_mut())
    .await
    .map_err(job_failed)?;

    let Some(updated) = updated else {
        return Err(job_failed(RepoError::from_persistence(format!(
            "post `{slug}` vanished during publish"
        ))));
    };

    tx.commit().await.map_err(job_failed)?;

    Ok(updated.id)
}

async fn mark_page_published(
    repositories: Arc<PostgresRepositories>,
    page_id: Uuid,
    publish_at: OffsetDateTime,
    slug: &str,
) -> Result<Uuid, ApalisError> {
    let mut tx = repositories.begin().await.map_err(job_failed)?;

    let updated = sqlx::query!(
        r#"
        UPDATE pages
           SET status = 'published'::page_status,
               published_at = $2,
               scheduled_at = NULL,
               updated_at = now()
         WHERE id = $1
        RETURNING id
        "#,
        page_id,
        publish_at
    )
    .fetch_optional(tx.as_mut())
    .await
    .map_err(job_failed)?;

    let Some(updated) = updated else {
        return Err(job_failed(RepoError::from_persistence(format!(
            "page `{slug}` vanished during publish"
        ))));
    };

    tx.commit().await.map_err(job_failed)?;

    Ok(updated.id)
}
