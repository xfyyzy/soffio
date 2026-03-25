use apalis::prelude::Error as ApalisError;
use tokio::task::JoinHandle;
use uuid::Uuid;

use crate::{
    application::{
        jobs::{JobWorkerContext, job_failed},
        repos::SettingsRepo,
    },
    infra::db::{PersistedPostSectionOwned, PostgresRepositories},
};

use super::{JobConsistencyError, RenderedSection};

pub(super) async fn persist_sections_and_summary(
    repos: &PostgresRepositories,
    post_id: Uuid,
    sections: &[PersistedPostSectionOwned],
    summary_html: Option<&str>,
) -> Result<(), ApalisError> {
    let mut tx = repos.begin().await.map_err(job_failed)?;

    // Lock posts row first to align lock order with snapshot rollbacks.
    repos
        .lock_post_for_update(&mut tx, post_id)
        .await
        .map_err(job_failed)?;

    repos
        .replace_post_sections_bulk(&mut tx, post_id, sections)
        .await
        .map_err(job_failed)?;

    if let Some(summary_html) = summary_html {
        repos
            .update_post_summary_html(&mut tx, post_id, summary_html)
            .await
            .map_err(job_failed)?;
    }

    repos
        .update_post_updated_at(&mut tx, post_id)
        .await
        .map_err(job_failed)?;

    tx.commit().await.map_err(job_failed)?;
    Ok(())
}

pub(super) async fn join_children(
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

pub(super) fn convert_section(
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

pub(super) async fn load_public_site_url(ctx: &JobWorkerContext) -> Result<String, ApalisError> {
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
