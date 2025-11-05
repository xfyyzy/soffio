use std::sync::Arc;

use serde::Serialize;
use thiserror::Error;
use time::OffsetDateTime;

use crate::application::admin::audit::AdminAuditService;
use crate::application::pagination::{CursorPage, JobCursor, PageRequest};
use crate::application::repos::{
    JobQueryFilter, JobsRepo, NewJobRecord, RepoError, UpdateJobStateParams,
};
use crate::domain::entities::JobRecord;
use crate::domain::types::{JobState, JobType};

#[derive(Debug, Error)]
pub enum AdminJobError {
    #[error("job not found")]
    NotFound,
    #[error(transparent)]
    Repo(#[from] RepoError),
}

#[derive(Debug, Clone)]
pub struct ScheduleJobCommand {
    pub job_type: JobType,
    pub payload: serde_json::Value,
    pub scheduled_at: Option<OffsetDateTime>,
    pub max_attempts: Option<i32>,
    pub priority: Option<i32>,
}

#[derive(Debug, Clone)]
pub struct UpdateJobStatusCommand {
    pub id: String,
    pub state: JobState,
    pub error_text: Option<String>,
    pub attempts: Option<i32>,
    pub run_at: Option<OffsetDateTime>,
    pub priority: Option<i32>,
}

#[derive(Clone)]
pub struct AdminJobService {
    repo: Arc<dyn JobsRepo>,
    audit: AdminAuditService,
}

impl AdminJobService {
    pub fn new(repo: Arc<dyn JobsRepo>, audit: AdminAuditService) -> Self {
        Self { repo, audit }
    }

    pub async fn schedule_job(
        &self,
        actor: &str,
        command: ScheduleJobCommand,
    ) -> Result<(), AdminJobError> {
        let run_at = command.scheduled_at.unwrap_or_else(OffsetDateTime::now_utc);
        let new_job = NewJobRecord {
            job_type: command.job_type,
            payload: command.payload,
            run_at,
            max_attempts: command.max_attempts.unwrap_or(25),
            priority: command.priority.unwrap_or(0),
        };

        let job_id = self.repo.enqueue_job(new_job).await?;

        let snapshot = JobSnapshot {
            id: job_id.clone(),
            job_type: command.job_type,
            run_at,
        };
        self.audit
            .record(actor, "job.enqueue", "job", Some(&job_id), Some(&snapshot))
            .await?;

        Ok(())
    }

    pub async fn list_jobs(
        &self,
        filter: &JobQueryFilter,
        page: PageRequest<JobCursor>,
    ) -> Result<CursorPage<JobRecord>, AdminJobError> {
        self.repo
            .list_jobs(filter, page)
            .await
            .map_err(AdminJobError::from)
    }

    pub async fn load_job(&self, id: &str) -> Result<JobRecord, AdminJobError> {
        self.repo.find_job(id).await?.ok_or(AdminJobError::NotFound)
    }

    pub async fn retry_job(&self, actor: &str, id: &str) -> Result<JobRecord, AdminJobError> {
        let job = self.load_job(id).await?;
        let command = UpdateJobStatusCommand {
            id: job.id.clone(),
            state: JobState::Pending,
            error_text: None,
            attempts: Some(0),
            run_at: Some(OffsetDateTime::now_utc()),
            priority: Some(job.priority),
        };

        self.update_status(actor, command).await?;
        self.load_job(id).await
    }

    pub async fn cancel_job(
        &self,
        actor: &str,
        id: &str,
        reason: Option<&str>,
    ) -> Result<JobRecord, AdminJobError> {
        let job = self.load_job(id).await?;
        let command = UpdateJobStatusCommand {
            id: job.id.clone(),
            state: JobState::Killed,
            error_text: reason.map(|value| value.to_string()),
            attempts: None,
            run_at: None,
            priority: None,
        };

        self.update_status(actor, command).await?;
        self.load_job(id).await
    }

    pub async fn update_status(
        &self,
        actor: &str,
        command: UpdateJobStatusCommand,
    ) -> Result<(), AdminJobError> {
        let params = UpdateJobStateParams {
            id: command.id.clone(),
            state: command.state,
            last_error: command.error_text.clone(),
            attempts: command.attempts,
            run_at: command.run_at,
            priority: command.priority,
        };

        self.repo.update_job_state(params).await?;

        let snapshot = JobStatusSnapshot {
            state: command.state,
            error_text: command.error_text.as_deref(),
        };
        self.audit
            .record(
                actor,
                "job.update_status",
                "job",
                Some(&command.id),
                Some(&snapshot),
            )
            .await?;
        Ok(())
    }
}

#[derive(Debug, Serialize)]
struct JobSnapshot {
    id: String,
    job_type: JobType,
    run_at: OffsetDateTime,
}

#[derive(Debug, Serialize)]
struct JobStatusSnapshot<'a> {
    state: JobState,
    #[serde(skip_serializing_if = "Option::is_none")]
    error_text: Option<&'a str>,
}
