use std::time::Duration;

use time::OffsetDateTime;
use tokio::time::{Instant, sleep};

use crate::{
    application::repos::{JobsRepo, NewJobRecord, RepoError},
    domain::types::{JobState, JobType},
};

const DEFAULT_WAIT_POLL_INTERVAL: Duration = Duration::from_millis(50);

/// Enqueue a job with the provided payload, returning the assigned ULID.
pub async fn enqueue_job<J, P>(
    repo: &J,
    job_type: JobType,
    payload: &P,
    run_at: Option<OffsetDateTime>,
    max_attempts: i32,
    priority: i32,
) -> Result<String, RepoError>
where
    J: JobsRepo + ?Sized,
    P: serde::Serialize,
{
    let payload = serde_json::to_value(payload)
        .map_err(|err| RepoError::from_persistence(err.to_string()))?;
    let record = NewJobRecord {
        job_type,
        payload,
        run_at: run_at.unwrap_or_else(OffsetDateTime::now_utc),
        max_attempts,
        priority,
    };

    repo.enqueue_job(record).await
}

/// Block until the job finishes or the timeout elapses, returning the final job snapshot.
pub async fn wait_for_job_completion<J>(
    repo: &J,
    job_id: &str,
    timeout: Duration,
) -> Result<crate::domain::entities::JobRecord, RepoError>
where
    J: JobsRepo + ?Sized,
{
    let deadline = Instant::now() + timeout;

    loop {
        let job = repo
            .find_job(job_id)
            .await?
            .ok_or_else(|| RepoError::from_persistence(format!("job `{job_id}` not found")))?;

        match job.state {
            JobState::Done => return Ok(job),
            JobState::Failed | JobState::Killed => {
                let message = job
                    .last_error
                    .unwrap_or_else(|| "job failed without error text".to_string());
                return Err(RepoError::from_persistence(message));
            }
            _ => {
                if Instant::now() >= deadline {
                    return Err(RepoError::from_persistence(format!(
                        "job `{job_id}` timed out after {:?}",
                        timeout
                    )));
                }

                sleep(DEFAULT_WAIT_POLL_INTERVAL).await;
            }
        }
    }
}
