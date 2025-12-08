use apalis::prelude::{Data, Error as ApalisError};
use serde::{Deserialize, Serialize};
use std::time::Instant;
use thiserror::Error;
use time::OffsetDateTime;
use tokio::task::JoinHandle;
use tracing::{info, warn};
use uuid::Uuid;

use crate::{
    application::{
        jobs::{JobWorkerContext, enqueue_job, job_failed},
        render::runtime::{InFlightError, RenderArtifact, RenderMailboxError},
        repos::{JobsRepo, RepoError, SettingsRepo},
    },
    domain::types::JobType,
    infra::db::PersistedPostSectionOwned,
};

use super::types::{RenderRequest, RenderService, RenderTarget, RenderedSection};

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenderPostJobPayload {
    pub slug: String,
    pub body_markdown: String,
    pub summary_markdown: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenderPostSectionsJobPayload {
    pub tracking_id: String,
    pub post_id: Uuid,
    pub slug: String,
    pub markdown: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenderPostSectionJobPayload {
    pub tracking_id: String,
    pub post_id: Uuid,
    pub slug: String,
    pub section: RenderedSection,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenderSummaryJobPayload {
    pub tracking_id: String,
    pub post_id: Uuid,
    pub slug: String,
    pub summary_markdown: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenderPageJobPayload {
    pub slug: String,
    pub markdown: String,
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

/// Container job responsible for coordinating render tasks and committing results.
pub async fn process_render_post_job(
    payload: RenderPostJobPayload,
    context: Data<JobWorkerContext>,
) -> Result<(), ApalisError> {
    let started_at = Instant::now();
    let ctx = &*context;
    let repositories = ctx.repositories.clone();

    // Use payload data directly to avoid race conditions with separate connection pools.
    // The body_markdown and summary_markdown were captured at enqueue time.
    let body_markdown = payload.body_markdown.clone();
    let summary_markdown = payload.summary_markdown.clone();

    // Only fetch post_id from database (immutable identifier).
    let Some(post_id) = sqlx::query_scalar!("SELECT id FROM posts WHERE slug = $1", payload.slug)
        .fetch_optional(repositories.pool())
        .await
        .map_err(job_failed)?
    else {
        return Err(job_failed(JobConsistencyError::new(format!(
            "post `{}` not found",
            payload.slug
        ))));
    };

    let guard = match ctx.inflight_renders.acquire(post_id) {
        Ok(guard) => guard,
        Err(InFlightError::AlreadyRunning { post_id }) => {
            warn!(
                target = "application::render::process_render_post_job",
                post_id = %post_id,
                slug = %payload.slug,
                "render already in flight; cancelling duplicate"
            );
            return Ok(());
        }
    };

    let mut child_handles: Vec<JoinHandle<Result<(), ApalisError>>> = Vec::new();

    let sections_tracking = Uuid::new_v4().to_string();
    let sections_receiver = ctx.render_mailbox.register(sections_tracking.clone());
    let sections_payload = RenderPostSectionsJobPayload {
        tracking_id: sections_tracking.clone(),
        post_id,
        slug: payload.slug.clone(),
        markdown: body_markdown,
    };
    let sections_ctx = ctx.clone();
    let sections_handle = tokio::spawn(async move {
        process_render_post_sections_job(sections_payload, Data::new(sections_ctx)).await
    });
    child_handles.push(sections_handle);

    let summary_state = if let Some(summary_markdown) = summary_markdown {
        let tracking_id = Uuid::new_v4().to_string();
        let receiver = ctx.render_mailbox.register(tracking_id.clone());
        let payload = RenderSummaryJobPayload {
            tracking_id: tracking_id.clone(),
            post_id,
            slug: payload.slug.clone(),
            summary_markdown,
        };
        let summary_ctx = ctx.clone();
        let handle = tokio::spawn(async move {
            process_render_summary_job(payload, Data::new(summary_ctx)).await
        });
        child_handles.push(handle);
        Some((tracking_id, receiver))
    } else {
        None
    };

    let sections_artifact = sections_receiver.await.map_err(|_| {
        job_failed(JobConsistencyError::new(
            "sections render result channel dropped",
        ))
    })?;

    let sections = match sections_artifact {
        RenderArtifact::Sections(sections) => sections,
        RenderArtifact::Cancelled(_) => {
            return Err(job_failed(JobConsistencyError::new(
                "sections render cancelled",
            )));
        }
        RenderArtifact::Section(_) | RenderArtifact::SummaryHtml(_) => {
            return Err(job_failed(JobConsistencyError::new(
                "unexpected render artifact variant for sections",
            )));
        }
    };

    let summary_html = if let Some((tracking_id, receiver)) = summary_state {
        let artifact = match receiver.await {
            Ok(artifact) => artifact,
            Err(_) => {
                ctx.render_mailbox.cancel(
                    &tracking_id,
                    RenderMailboxError::Aborted(
                        "summary render result channel dropped".to_string(),
                    ),
                );
                return Err(job_failed(JobConsistencyError::new(
                    "summary render result channel dropped",
                )));
            }
        };
        match artifact {
            RenderArtifact::SummaryHtml(html) => Some(html),
            RenderArtifact::Cancelled(_) => None,
            _ => {
                return Err(job_failed(JobConsistencyError::new(
                    "unexpected render artifact variant for summary",
                )));
            }
        }
    } else {
        None
    };

    persist_sections_and_summary(ctx, post_id, &sections, summary_html.as_deref()).await?;

    drop(guard);

    join_children(child_handles).await?;

    info!(
        target = "application::render::process_render_post_job",
        slug = %payload.slug,
        sections = sections.len(),
        summary = summary_html.is_some(),
        elapsed_ms = started_at.elapsed().as_millis() as u64,
        "post render persisted"
    );

    Ok(())
}

async fn persist_sections_and_summary(
    ctx: &JobWorkerContext,
    post_id: Uuid,
    sections: &[PersistedPostSectionOwned],
    summary_html: Option<&str>,
) -> Result<(), ApalisError> {
    let mut tx = ctx.repositories.begin().await.map_err(job_failed)?;

    ctx.repositories
        .replace_post_sections_bulk(&mut tx, post_id, sections)
        .await
        .map_err(job_failed)?;

    if let Some(summary_html) = summary_html {
        ctx.repositories
            .update_post_summary_html(&mut tx, post_id, summary_html)
            .await
            .map_err(job_failed)?;
    }

    ctx.repositories
        .update_post_updated_at(&mut tx, post_id)
        .await
        .map_err(job_failed)?;

    tx.commit().await.map_err(job_failed)?;
    Ok(())
}

/// Container job that spawns per-section render sub-jobs.
pub async fn process_render_post_sections_job(
    payload: RenderPostSectionsJobPayload,
    context: Data<JobWorkerContext>,
) -> Result<(), ApalisError> {
    let started_at = Instant::now();
    let ctx = &*context;
    let renderer = ctx.renderer.clone();
    let tracking_id = payload.tracking_id.clone();
    let public_site_url = load_public_site_url(ctx).await?;

    let request = RenderRequest::new(
        RenderTarget::PostBody {
            slug: payload.slug.clone(),
        },
        payload.markdown.clone(),
    )
    .with_public_site_url(&public_site_url);

    let output = renderer.render(&request).map_err(job_failed)?;

    let sections = output.sections.unwrap_or_default();

    let mut receivers = Vec::with_capacity(sections.len());
    let mut handles: Vec<JoinHandle<Result<(), ApalisError>>> = Vec::with_capacity(sections.len());

    for (idx, section) in sections.iter().enumerate() {
        let child_tracking = format!("{}:{}", tracking_id, idx);
        let receiver = ctx.render_mailbox.register(child_tracking.clone());
        receivers.push(receiver);

        let payload = RenderPostSectionJobPayload {
            tracking_id: child_tracking,
            post_id: payload.post_id,
            slug: payload.slug.clone(),
            section: section.clone(),
        };
        let child_ctx = ctx.clone();
        let handle = tokio::spawn(async move {
            process_render_post_section_job(payload, Data::new(child_ctx)).await
        });
        handles.push(handle);
    }

    let mut persisted_sections = Vec::with_capacity(sections.len());
    for receiver in receivers {
        let artifact = receiver.await.map_err(|_| {
            job_failed(JobConsistencyError::new(
                "section render result channel dropped",
            ))
        })?;

        match artifact {
            RenderArtifact::Section(section) => persisted_sections.push(section),
            RenderArtifact::Cancelled(_) => continue,
            _ => {
                return Err(job_failed(JobConsistencyError::new(
                    "unexpected artifact variant for section",
                )));
            }
        }
    }

    join_children(handles).await?;

    info!(
        target = "application::render::process_render_post_sections_job",
        slug = %payload.slug,
        post_id = %payload.post_id,
        sections = persisted_sections.len(),
        elapsed_ms = started_at.elapsed().as_millis() as u64,
        "sections rendered"
    );

    ctx.render_mailbox
        .deliver(&tracking_id, RenderArtifact::Sections(persisted_sections))
        .map_err(|err| job_failed(JobConsistencyError::new(err.to_string())))
}

/// Leaf job: converts a rendered section into a persistable artifact.
pub async fn process_render_post_section_job(
    payload: RenderPostSectionJobPayload,
    context: Data<JobWorkerContext>,
) -> Result<(), ApalisError> {
    let tracking_id = payload.tracking_id.clone();
    let ctx = &*context;

    let result = convert_section(&payload.section, &payload.slug);

    match result {
        Ok(persisted) => ctx
            .render_mailbox
            .deliver(&tracking_id, RenderArtifact::Section(persisted))
            .map_err(|err| job_failed(JobConsistencyError::new(err.to_string()))),
        Err(err) => {
            let reason = RenderMailboxError::Aborted(err.to_string());
            ctx.render_mailbox.cancel(&tracking_id, reason.clone());
            Err(err)
        }
    }
}

/// Leaf job responsible for rendering the summary markdown.
pub async fn process_render_summary_job(
    payload: RenderSummaryJobPayload,
    context: Data<JobWorkerContext>,
) -> Result<(), ApalisError> {
    let ctx = &*context;
    let renderer = ctx.renderer.clone();
    let tracking_id = payload.tracking_id.clone();
    let started_at = Instant::now();
    let public_site_url = load_public_site_url(ctx).await?;

    let result = renderer
        .render(
            &RenderRequest::new(
                RenderTarget::PostSummary {
                    slug: payload.slug.clone(),
                },
                payload.summary_markdown,
            )
            .with_public_site_url(&public_site_url),
        )
        .map_err(job_failed);

    match result {
        Ok(output) => {
            ctx.render_mailbox
                .deliver(&tracking_id, RenderArtifact::SummaryHtml(output.html))
                .map_err(|err| job_failed(JobConsistencyError::new(err.to_string())))?;

            info!(
                target = "application::render::process_render_summary_job",
                slug = %payload.slug,
                elapsed_ms = started_at.elapsed().as_millis() as u64,
                "summary rendered"
            );

            Ok(())
        }
        Err(err) => {
            let reason = RenderMailboxError::Aborted(err.to_string());
            ctx.render_mailbox.cancel(&tracking_id, reason.clone());
            Err(err)
        }
    }
}

pub async fn process_render_page_job(
    payload: RenderPageJobPayload,
    context: Data<JobWorkerContext>,
) -> Result<(), ApalisError> {
    let ctx = &*context;
    let renderer = ctx.renderer.clone();
    let public_site_url = load_public_site_url(ctx).await?;

    let Some(page_id) = ctx
        .repositories
        .find_page_id_by_slug_immediate(&payload.slug)
        .await
        .map_err(job_failed)?
    else {
        return Err(job_failed(JobConsistencyError::new(format!(
            "page `{}` not found",
            payload.slug
        ))));
    };

    let request = RenderRequest::new(
        RenderTarget::PageBody {
            slug: payload.slug.clone(),
        },
        payload.markdown,
    )
    .with_public_site_url(&public_site_url);

    let output = renderer.render(&request).map_err(job_failed)?;

    let mut tx = ctx.repositories.begin().await.map_err(job_failed)?;
    ctx.repositories
        .update_page_rendered_html(&mut tx, page_id, &output.html)
        .await
        .map_err(job_failed)?;

    tx.commit().await.map_err(job_failed)?;
    Ok(())
}

async fn join_children(
    handles: Vec<JoinHandle<Result<(), ApalisError>>>,
) -> Result<(), ApalisError> {
    for handle in handles {
        match handle.await {
            Ok(result) => result?,
            Err(err) => {
                return Err(job_failed(JobConsistencyError::new(format!(
                    "render child task panicked: {err}",
                ))));
            }
        }
    }
    Ok(())
}

fn convert_section(
    section: &RenderedSection,
    slug: &str,
) -> Result<PersistedPostSectionOwned, ApalisError> {
    let position = i32::try_from(section.position).map_err(|_| {
        job_failed(JobConsistencyError::new(format!(
            "section position overflow for `{slug}`"
        )))
    })?;

    Ok(PersistedPostSectionOwned {
        id: section.id,
        parent_id: section.parent_id,
        position,
        level: i16::from(section.level),
        heading_html: section.heading_html.clone(),
        heading_text: section.heading_text.clone(),
        body_html: section.body_html.clone(),
        contains_code: section.contains_code,
        contains_math: section.contains_math,
        contains_mermaid: section.contains_mermaid,
        anchor_slug: section.anchor_slug.clone(),
    })
}

async fn load_public_site_url(ctx: &JobWorkerContext) -> Result<String, ApalisError> {
    let settings = ctx
        .repositories
        .load_site_settings()
        .await
        .map_err(job_failed)?;
    Ok(normalize_public_site_url(&settings.public_site_url))
}

fn normalize_public_site_url(url: &str) -> String {
    let trimmed = url.trim();
    if trimmed.is_empty() {
        return String::new();
    }

    let without_trailing = trimmed.trim_end_matches('/');
    format!("{without_trailing}/")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Verifies that RenderPostJobPayload correctly serializes and deserializes
    /// body_markdown and summary_markdown fields. This is critical for the race
    /// condition fix: the payload must carry complete content so the worker
    /// doesn't need to re-read from the database.
    #[test]
    fn render_post_payload_carries_complete_content() {
        let payload = RenderPostJobPayload {
            slug: "test-post".into(),
            body_markdown: "# Heading\n\nParagraph with **bold** text.".into(),
            summary_markdown: Some("Summary content here.".into()),
        };

        let json = serde_json::to_string(&payload).unwrap();
        let deserialized: RenderPostJobPayload = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.slug, "test-post");
        assert_eq!(
            deserialized.body_markdown,
            "# Heading\n\nParagraph with **bold** text."
        );
        assert_eq!(
            deserialized.summary_markdown,
            Some("Summary content here.".into())
        );
    }

    /// Verifies that RenderPostJobPayload handles None summary_markdown correctly.
    #[test]
    fn render_post_payload_handles_none_summary() {
        let payload = RenderPostJobPayload {
            slug: "no-summary".into(),
            body_markdown: "Body only".into(),
            summary_markdown: None,
        };

        let json = serde_json::to_string(&payload).unwrap();
        let deserialized: RenderPostJobPayload = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.summary_markdown, None);
    }

    /// Verifies that large markdown content is preserved through serialization.
    #[test]
    fn render_post_payload_preserves_large_content() {
        let large_body = "# Title\n\n".to_string() + &"Lorem ipsum dolor sit amet. ".repeat(1000);
        let payload = RenderPostJobPayload {
            slug: "large-post".into(),
            body_markdown: large_body.clone(),
            summary_markdown: Some("Short summary".into()),
        };

        let json = serde_json::to_string(&payload).unwrap();
        let deserialized: RenderPostJobPayload = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.body_markdown, large_body);
    }
}

