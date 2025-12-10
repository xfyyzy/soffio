use apalis::prelude::{Data, Error as ApalisError};
use serde::{Deserialize, Serialize};
use tracing::{info, warn};

use crate::{
    application::repos::{JobsRepo, RepoError},
    infra::{
        cache::{CacheWarmDebouncer, ResponseCache},
        cache_warmer::CacheWarmer,
        http::HttpState,
    },
};

use super::{context::JobWorkerContext, queue::enqueue_job};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CacheWarmJobPayload {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    pub epoch: u64,
}

/// Enqueue an async cache warm job.
/// This is fire-and-forget; the caller does not wait for completion.
pub async fn enqueue_cache_warm_job<J: JobsRepo + ?Sized>(
    repo: &J,
    reason: Option<String>,
    epoch: u64,
) -> Result<String, RepoError> {
    let payload = CacheWarmJobPayload { reason, epoch };
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

/// Single entry point for write paths: synchronously invalidate cache, then
/// (debounced) enqueue a warm-cache job. Returns Some(job_id) if enqueued.
pub async fn invalidate_and_enqueue_warm(
    cache: &ResponseCache,
    debouncer: &CacheWarmDebouncer,
    jobs_repo: &dyn JobsRepo,
    reason: Option<String>,
) -> Result<Option<String>, RepoError> {
    cache.invalidate_all().await;
    let epoch = cache.epoch();

    if debouncer.try_warm().await {
        let job_id = enqueue_cache_warm_job(jobs_repo, reason, epoch).await?;
        debouncer.mark_warm_requested().await;
        Ok(Some(job_id))
    } else {
        Ok(None)
    }
}

/// Process a cache warm job.
/// This job pre-warms commonly accessed pages after cache invalidation.
pub async fn process_cache_warm_job(
    payload: CacheWarmJobPayload,
    context: Data<JobWorkerContext>,
) -> Result<(), ApalisError> {
    let ctx = &*context;

    // Abort if cache has been invalidated since this job was enqueued.
    let current_epoch = ctx.cache.epoch();
    if current_epoch > payload.epoch {
        info!(
            target = "application::jobs::process_cache_warm_job",
            reason = payload.reason.as_deref().unwrap_or("unspecified"),
            job_epoch = payload.epoch,
            cache_epoch = current_epoch,
            "skipping warm cache job; cache epoch moved forward"
        );
        return Ok(());
    }

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
