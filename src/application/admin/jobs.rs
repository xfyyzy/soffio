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

    /// Count jobs grouped by state.
    pub async fn status_counts(
        &self,
        base_filter: &JobQueryFilter,
    ) -> Result<AdminJobStatusCounts, AdminJobError> {
        // Pre-build filters to avoid temporary value borrow issues with try_join!
        let filter_total = JobQueryFilter {
            state: None,
            ..(base_filter.clone())
        };
        let filter_pending = JobQueryFilter {
            state: Some(JobState::Pending),
            ..(base_filter.clone())
        };
        let filter_scheduled = JobQueryFilter {
            state: Some(JobState::Scheduled),
            ..(base_filter.clone())
        };
        let filter_running = JobQueryFilter {
            state: Some(JobState::Running),
            ..(base_filter.clone())
        };
        let filter_done = JobQueryFilter {
            state: Some(JobState::Done),
            ..(base_filter.clone())
        };
        let filter_failed = JobQueryFilter {
            state: Some(JobState::Failed),
            ..(base_filter.clone())
        };
        let filter_killed = JobQueryFilter {
            state: Some(JobState::Killed),
            ..(base_filter.clone())
        };

        let (total, pending, scheduled, running, done, failed, killed) = tokio::try_join!(
            self.repo.count_jobs(&filter_total),
            self.repo.count_jobs(&filter_pending),
            self.repo.count_jobs(&filter_scheduled),
            self.repo.count_jobs(&filter_running),
            self.repo.count_jobs(&filter_done),
            self.repo.count_jobs(&filter_failed),
            self.repo.count_jobs(&filter_killed),
        )?;

        Ok(AdminJobStatusCounts {
            total,
            pending: pending + scheduled, // Combine Pending and Scheduled
            running,
            done,
            failed,
            killed,
        })
    }

    /// Count jobs grouped by job type.
    pub async fn type_counts(
        &self,
        base_filter: &JobQueryFilter,
    ) -> Result<AdminJobTypeCounts, AdminJobError> {
        // Pre-build filters to avoid temporary value borrow issues with try_join!
        let filter_render_post = JobQueryFilter {
            job_type: Some(JobType::RenderPost),
            ..(base_filter.clone())
        };
        let filter_render_post_sections = JobQueryFilter {
            job_type: Some(JobType::RenderPostSections),
            ..(base_filter.clone())
        };
        let filter_render_post_section = JobQueryFilter {
            job_type: Some(JobType::RenderPostSection),
            ..(base_filter.clone())
        };
        let filter_render_page = JobQueryFilter {
            job_type: Some(JobType::RenderPage),
            ..(base_filter.clone())
        };
        let filter_render_summary = JobQueryFilter {
            job_type: Some(JobType::RenderSummary),
            ..(base_filter.clone())
        };
        let filter_publish_post = JobQueryFilter {
            job_type: Some(JobType::PublishPost),
            ..(base_filter.clone())
        };
        let filter_publish_page = JobQueryFilter {
            job_type: Some(JobType::PublishPage),
            ..(base_filter.clone())
        };
        let filter_invalidate_cache = JobQueryFilter {
            job_type: Some(JobType::InvalidateCache),
            ..(base_filter.clone())
        };
        let filter_warm_cache = JobQueryFilter {
            job_type: Some(JobType::WarmCache),
            ..(base_filter.clone())
        };

        let (
            render_post,
            render_post_sections,
            render_post_section,
            render_page,
            render_summary,
            publish_post,
            publish_page,
            invalidate_cache,
            warm_cache,
        ) = tokio::try_join!(
            self.repo.count_jobs(&filter_render_post),
            self.repo.count_jobs(&filter_render_post_sections),
            self.repo.count_jobs(&filter_render_post_section),
            self.repo.count_jobs(&filter_render_page),
            self.repo.count_jobs(&filter_render_summary),
            self.repo.count_jobs(&filter_publish_post),
            self.repo.count_jobs(&filter_publish_page),
            self.repo.count_jobs(&filter_invalidate_cache),
            self.repo.count_jobs(&filter_warm_cache),
        )?;

        Ok(AdminJobTypeCounts {
            render_post,
            render_post_sections,
            render_post_section,
            render_page,
            render_summary,
            publish_post,
            publish_page,
            invalidate_cache,
            warm_cache,
        })
    }
}

/// Status counts for job state filters.
#[derive(Debug, Clone)]
pub struct AdminJobStatusCounts {
    pub total: u64,
    pub pending: u64,
    pub running: u64,
    pub done: u64,
    pub failed: u64,
    pub killed: u64,
}

/// Type counts for job type filter dropdown.
#[derive(Debug, Clone)]
pub struct AdminJobTypeCounts {
    pub render_post: u64,
    pub render_post_sections: u64,
    pub render_post_section: u64,
    pub render_page: u64,
    pub render_summary: u64,
    pub publish_post: u64,
    pub publish_page: u64,
    pub invalidate_cache: u64,
    pub warm_cache: u64,
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
