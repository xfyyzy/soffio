use apalis::prelude::{Data, Error as ApalisError};
use serde::{Deserialize, Serialize};
use tracing::{info, warn};

use crate::{
    application::repos::{JobsRepo, RepoError},
    infra::{cache_warmer::CacheWarmer, http::HttpState},
};

use super::{context::JobWorkerContext, queue::enqueue_job};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CacheWarmJobPayload {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

/// Enqueue an async cache warm job.
/// This is fire-and-forget; the caller does not wait for completion.
pub async fn enqueue_cache_warm_job<J: JobsRepo + ?Sized>(
    repo: &J,
    reason: Option<String>,
) -> Result<String, RepoError> {
    let payload = CacheWarmJobPayload { reason };
    enqueue_job(
        repo,
        crate::domain::types::JobType::WarmCache,
        &payload,
        None,
        1, // low priority
        3, // fewer retries since it's best-effort
    )
    .await
}

/// Process a cache warm job.
/// This job pre-warms commonly accessed pages after cache invalidation.
pub async fn process_cache_warm_job(
    payload: CacheWarmJobPayload,
    context: Data<JobWorkerContext>,
) -> Result<(), ApalisError> {
    let ctx = &*context;

    info!(
        target = "application::jobs::process_cache_warm_job",
        reason = payload.reason.as_deref().unwrap_or("unspecified"),
        "starting cache warm"
    );

    let state = HttpState {
        feed: ctx.feed.clone(),
        pages: ctx.pages.clone(),
        chrome: ctx.chrome.clone(),
        cache: ctx.cache.clone(),
        db: ctx.repositories.clone(),
        upload_storage: ctx.upload_storage.clone(),
    };

    if let Err(err) = CacheWarmer::new(state).warm_initial().await {
        warn!(
            target = "application::jobs::process_cache_warm_job",
            error = %err,
            "cache warm failed"
        );
        // Don't return error - warming is best-effort
    } else {
        info!(
            target = "application::jobs::process_cache_warm_job",
            "cache warm completed"
        );
    }

    Ok(())
}
