use async_trait::async_trait;
use time::OffsetDateTime;

use crate::application::pagination::{CursorPage, JobCursor, PageRequest};
use crate::domain::entities::JobRecord;
use crate::domain::types::{JobState, JobType};

use super::RepoError;

#[derive(Debug, Clone, Default)]
pub struct JobQueryFilter {
    pub state: Option<JobState>,
    pub job_type: Option<JobType>,
    pub search: Option<String>,
}

#[derive(Debug, Clone)]
pub struct NewJobRecord {
    pub job_type: JobType,
    pub payload: serde_json::Value,
    pub run_at: OffsetDateTime,
    pub max_attempts: i32,
    pub priority: i32,
}

#[derive(Debug, Clone)]
pub struct UpdateJobStateParams {
    pub id: String,
    pub state: JobState,
    pub last_error: Option<String>,
    pub attempts: Option<i32>,
    pub run_at: Option<OffsetDateTime>,
    pub priority: Option<i32>,
}

#[async_trait]
pub trait JobsRepo: Send + Sync {
    async fn enqueue_job(&self, job: NewJobRecord) -> Result<String, RepoError>;

    async fn update_job_state(&self, params: UpdateJobStateParams) -> Result<(), RepoError>;

    async fn find_job(&self, id: &str) -> Result<Option<JobRecord>, RepoError>;

    async fn list_jobs(
        &self,
        filter: &JobQueryFilter,
        page: PageRequest<JobCursor>,
    ) -> Result<CursorPage<JobRecord>, RepoError>;

    async fn count_jobs(&self, filter: &JobQueryFilter) -> Result<u64, RepoError>;
}
