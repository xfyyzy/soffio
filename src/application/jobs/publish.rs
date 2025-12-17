use apalis::prelude::{Data, Error as ApalisError};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use tracing::info;

use crate::{
    application::repos::{JobsRepo, RepoError},
    domain::types::JobType,
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
    ctx.admin_posts
        .publish_scheduled_by_slug(&payload.slug)
        .await
        .map_err(job_failed)?;

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
    ctx.admin_pages
        .publish_scheduled_by_slug(&payload.slug)
        .await
        .map_err(job_failed)?;

    info!(
        target = "application::jobs::process_publish_page_job",
        slug = payload.slug,
        "page published"
    );

    Ok(())
}
