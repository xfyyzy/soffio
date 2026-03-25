use thiserror::Error;
use time::OffsetDateTime;

use crate::application::jobs::enqueue_job;
use crate::application::repos::{JobsRepo, RepoError};
use crate::domain::types::JobType;

use super::types::RenderedSection;

mod helpers;
mod payloads;
mod process;
#[cfg(test)]
mod tests;

pub use payloads::{
    RenderPageJobPayload, RenderPostJobPayload, RenderPostSectionJobPayload,
    RenderPostSectionsJobPayload, RenderSummaryJobPayload,
};
pub use process::{
    process_render_page_job, process_render_post_job, process_render_post_section_job,
    process_render_post_sections_job, process_render_summary_job,
};

/// Schedules a top-level post render container job.
///
/// The payload carries `body_markdown` and `summary_markdown` inline to avoid
/// race conditions: the job worker uses these values directly instead of
/// re-reading from the database (which might return stale data due to separate
/// connection pools).
pub async fn enqueue_render_post_job<J: JobsRepo + ?Sized>(
    repo: &J,
    slug: String,
    body_markdown: String,
    summary_markdown: Option<String>,
    scheduled_at: Option<OffsetDateTime>,
) -> Result<String, RepoError> {
    enqueue_job(
        repo,
        JobType::RenderPost,
        &RenderPostJobPayload {
            slug,
            body_markdown,
            summary_markdown,
        },
        scheduled_at,
        25,
        0,
    )
    .await
}

pub async fn enqueue_render_page_job<J: JobsRepo + ?Sized>(
    repo: &J,
    slug: String,
    markdown: String,
    scheduled_at: Option<OffsetDateTime>,
) -> Result<String, RepoError> {
    enqueue_job(
        repo,
        JobType::RenderPage,
        &RenderPageJobPayload { slug, markdown },
        scheduled_at,
        25,
        0,
    )
    .await
}

#[derive(Debug, Error)]
#[error("{message}")]
struct JobConsistencyError {
    message: String,
}

impl JobConsistencyError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}
