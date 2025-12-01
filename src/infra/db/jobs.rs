use std::convert::TryFrom;

use async_trait::async_trait;
use sqlx::QueryBuilder;
use time::OffsetDateTime;

use crate::{
    application::pagination::{CursorPage, JobCursor, PageRequest},
    application::repos::{JobQueryFilter, JobsRepo, NewJobRecord, RepoError, UpdateJobStateParams},
    domain::{
        entities::JobRecord,
        types::{JobState, JobType},
    },
};

use super::{PostgresRepositories, map_sqlx_error};

#[derive(sqlx::FromRow)]
struct JobRow {
    id: String,
    job_type: String,
    job: serde_json::Value,
    status: String,
    attempts: i32,
    max_attempts: i32,
    run_at: OffsetDateTime,
    last_error: Option<String>,
    lock_at: Option<OffsetDateTime>,
    lock_by: Option<String>,
    done_at: Option<OffsetDateTime>,
    priority: Option<i32>,
}

impl TryFrom<JobRow> for JobRecord {
    type Error = RepoError;

    fn try_from(row: JobRow) -> Result<Self, Self::Error> {
        let job_type = JobType::try_from(row.job_type.as_str()).map_err(|_| {
            RepoError::from_persistence(format!("unknown job type `{}`", row.job_type))
        })?;

        let state = JobState::try_from(row.status.as_str()).map_err(|_| {
            RepoError::from_persistence(format!("unknown job state `{}`", row.status))
        })?;

        Ok(Self {
            id: row.id,
            job_type,
            payload: row.job,
            state,
            attempts: row.attempts,
            max_attempts: row.max_attempts,
            run_at: row.run_at,
            lock_at: row.lock_at,
            lock_by: row.lock_by,
            done_at: row.done_at,
            last_error: row.last_error,
            priority: row.priority.unwrap_or(0),
        })
    }
}

#[async_trait]
impl JobsRepo for PostgresRepositories {
    async fn enqueue_job(&self, job: NewJobRecord) -> Result<String, RepoError> {
        let record = sqlx::query!(
            r#"
            SELECT (apalis.push_job($1, $2::json, $3, $4, $5, $6)).id AS "id!"
            "#,
            job.job_type.as_str(),
            job.payload,
            "Pending",
            job.run_at,
            job.max_attempts,
            job.priority
        )
        .fetch_one(self.pool())
        .await
        .map_err(map_sqlx_error)?;

        Ok(record.id)
    }

    async fn update_job_state(&self, params: UpdateJobStateParams) -> Result<(), RepoError> {
        let UpdateJobStateParams {
            id,
            state,
            last_error,
            attempts,
            run_at,
            priority,
        } = params;

        let error_ref = last_error.as_deref();
        sqlx::query!(
            r#"
            UPDATE apalis.jobs
               SET status = $2,
                   last_error = $3,
                   attempts = COALESCE($4, attempts),
                   run_at = COALESCE($5, run_at),
                   priority = COALESCE($6, priority),
                   done_at = CASE
                       WHEN $2 IN ('Done','Failed','Killed') THEN COALESCE(done_at, now())
                       ELSE NULL
                   END
             WHERE id = $1
            "#,
            &id,
            state.as_str(),
            error_ref,
            attempts,
            run_at,
            priority
        )
        .execute(self.pool())
        .await
        .map_err(map_sqlx_error)?;

        Ok(())
    }

    async fn find_job(&self, id: &str) -> Result<Option<JobRecord>, RepoError> {
        let row = sqlx::query_as!(
            JobRow,
            r#"
            SELECT id,
                   job_type,
                   job,
                   status,
                   attempts,
                   max_attempts,
                   run_at,
                   last_error,
                   lock_at,
                   lock_by,
                   done_at,
                   priority
              FROM apalis.jobs
             WHERE id = $1
            "#,
            id
        )
        .fetch_optional(self.pool())
        .await
        .map_err(map_sqlx_error)?;

        match row {
            Some(row) => JobRecord::try_from(row).map(Some),
            None => Ok(None),
        }
    }

    async fn list_jobs(
        &self,
        filter: &JobQueryFilter,
        page: PageRequest<JobCursor>,
    ) -> Result<CursorPage<JobRecord>, RepoError> {
        let limit = page.limit.clamp(1, 200);
        let mut qb = QueryBuilder::new(
            "SELECT id,
                    job_type,
                    job,
                    status,
                    attempts,
                    max_attempts,
                    run_at,
                    last_error,
                    lock_at,
                    lock_by,
                    done_at,
                    priority
              FROM apalis.jobs
             WHERE 1=1 ",
        );

        if let Some(state) = filter.state {
            qb.push("AND status = ");
            qb.push_bind(state.as_str());
        }

        if let Some(job_type) = filter.job_type {
            qb.push(" AND job_type = ");
            qb.push_bind(job_type.as_str());
        }

        if let Some(search) = filter.search.as_ref() {
            let pattern = format!("%{}%", search);
            qb.push(" AND (");
            qb.push("id ILIKE ");
            qb.push_bind(pattern.clone());
            qb.push(" OR job_type ILIKE ");
            qb.push_bind(pattern.clone());
            qb.push(" OR status ILIKE ");
            qb.push_bind(pattern.clone());
            qb.push(" OR last_error ILIKE ");
            qb.push_bind(pattern);
            qb.push(")");
        }

        if let Some(cursor) = page.cursor {
            let run_at = cursor.run_at();
            let cursor_id = cursor.id().to_string();
            qb.push(" AND (");
            qb.push("run_at < ");
            qb.push_bind(run_at);
            qb.push(" OR (run_at = ");
            qb.push_bind(run_at);
            qb.push(" AND id < ");
            qb.push_bind(cursor_id);
            qb.push("))");
        }

        qb.push(" ORDER BY run_at DESC, id DESC ");
        qb.push("LIMIT ");
        qb.push_bind(limit as i64);

        let rows = qb
            .build_query_as::<JobRow>()
            .fetch_all(self.pool())
            .await
            .map_err(map_sqlx_error)?;
        let mut records = Vec::with_capacity(rows.len());
        for row in rows {
            records.push(JobRecord::try_from(row)?);
        }

        let next_cursor = if records.len() as u32 == limit {
            records
                .last()
                .map(|job| JobCursor::new(job.run_at, job.id.clone()).encode())
        } else {
            None
        };

        Ok(CursorPage::new(records, next_cursor))
    }
}
