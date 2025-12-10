//! Job state and type parsing utilities.

use axum::http::StatusCode;

use crate::{
    application::error::HttpError,
    domain::types::{JobState, JobType},
    presentation::admin::views as admin_views,
};

/// Parse job state from optional string parameter.
pub(super) fn parse_job_state(value: Option<&str>) -> Result<Option<JobState>, HttpError> {
    let Some(raw) = value else {
        return Ok(None);
    };

    if raw.is_empty() {
        return Ok(None);
    }

    match raw.to_ascii_lowercase().as_str() {
        "pending" => Ok(Some(JobState::Pending)),
        "scheduled" => Ok(Some(JobState::Scheduled)),
        "running" => Ok(Some(JobState::Running)),
        "done" => Ok(Some(JobState::Done)),
        "failed" => Ok(Some(JobState::Failed)),
        "killed" => Ok(Some(JobState::Killed)),
        other => Err(HttpError::new(
            "infra::http::admin::jobs::parse_job_state",
            StatusCode::BAD_REQUEST,
            "Unknown job state filter",
            format!("State `{other}` is not recognised"),
        )),
    }
}

/// Parse job type from optional string parameter.
pub(super) fn parse_job_type(value: Option<&str>) -> Result<Option<JobType>, HttpError> {
    let Some(raw) = value else {
        return Ok(None);
    };

    if raw.is_empty() {
        return Ok(None);
    }

    JobType::try_from(raw).map(Some).map_err(|()| {
        HttpError::new(
            "infra::http::admin::jobs::parse_job_type",
            StatusCode::BAD_REQUEST,
            "Unknown job type filter",
            format!("Job type `{raw}` is not recognised"),
        )
    })
}

use crate::application::admin::jobs::AdminJobStatusCounts;

/// Build status filter tabs from counts.
pub(super) fn status_filters(
    counts: &AdminJobStatusCounts,
    active: Option<JobState>,
) -> Vec<admin_views::AdminJobStatusFilterView> {
    let mut filters = Vec::new();

    filters.push(admin_views::AdminJobStatusFilterView {
        status_key: None,
        label: "All".to_string(),
        count: counts.total,
        is_active: active.is_none(),
    });

    for state in [
        JobState::Pending,
        JobState::Running,
        JobState::Done,
        JobState::Failed,
        JobState::Killed,
    ] {
        let count = match state {
            JobState::Pending => counts.pending,
            JobState::Running => counts.running,
            JobState::Done => counts.done,
            JobState::Failed => counts.failed,
            JobState::Killed => counts.killed,
            JobState::Scheduled => 0,
        };
        filters.push(admin_views::AdminJobStatusFilterView {
            status_key: Some(state_key(state).to_string()),
            label: state_label(state).to_string(),
            count,
            is_active: active == Some(state),
        });
    }

    filters
}

/// Convert job state to lowercase key for badge styling.
pub(super) fn state_key(state: JobState) -> &'static str {
    match state {
        JobState::Pending => "pending",
        JobState::Scheduled => "scheduled",
        JobState::Running => "running",
        JobState::Done => "done",
        JobState::Failed => "failed",
        JobState::Killed => "killed",
    }
}

/// Get display label for job state.
pub(super) fn state_label(state: JobState) -> &'static str {
    state.as_str()
}

/// Convert job type to snake_case key for badge styling.
pub(super) fn job_type_key(job_type: JobType) -> &'static str {
    job_type.as_str()
}

/// Get display label for job type.
pub(super) fn job_type_label(job_type: JobType) -> &'static str {
    match job_type {
        JobType::RenderPost => "Render Post",
        JobType::RenderPostSections => "Render Sections",
        JobType::RenderPostSection => "Render Section",
        JobType::RenderPage => "Render Page",
        JobType::RenderSummary => "Render Summary",
        JobType::PublishPost => "Publish Post",
        JobType::PublishPage => "Publish Page",
        JobType::InvalidateCache => "Invalidate Cache",
        JobType::WarmCache => "Warm Cache",
    }
}

use crate::application::admin::jobs::AdminJobTypeCounts;

/// Build job type filter options for the dropdown with counts.
pub(super) fn job_type_options(
    counts: &AdminJobTypeCounts,
) -> Vec<admin_views::AdminJobTypeOption> {
    [
        (JobType::RenderPost, counts.render_post),
        (JobType::RenderPage, counts.render_page),
        (JobType::PublishPost, counts.publish_post),
        (JobType::PublishPage, counts.publish_page),
        (JobType::InvalidateCache, counts.invalidate_cache),
        (JobType::WarmCache, counts.warm_cache),
    ]
    .into_iter()
    .map(|(job_type, count)| admin_views::AdminJobTypeOption {
        value: job_type_key(job_type).to_string(),
        label: job_type_label(job_type).to_string(),
        count,
    })
    .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn job_type_options_expose_only_enqueued_jobs() {
        let counts = AdminJobTypeCounts {
            render_post: 1,
            render_page: 2,
            publish_post: 3,
            publish_page: 4,
            invalidate_cache: 5,
            warm_cache: 6,
        };

        let options = job_type_options(&counts);

        let keys: Vec<&str> = options.iter().map(|opt| opt.value.as_str()).collect();
        assert_eq!(
            keys,
            vec![
                JobType::RenderPost.as_str(),
                JobType::RenderPage.as_str(),
                JobType::PublishPost.as_str(),
                JobType::PublishPage.as_str(),
                JobType::InvalidateCache.as_str(),
                JobType::WarmCache.as_str(),
            ]
        );

        let counts: Vec<u64> = options.iter().map(|opt| opt.count).collect();
        assert_eq!(counts, vec![1, 2, 3, 4, 5, 6]);
    }
}
