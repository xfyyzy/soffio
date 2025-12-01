use axum::{
    extract::{Form, Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Redirect, Response},
};
use serde::Deserialize;
use serde_json::to_string_pretty;
use url::form_urlencoded::Serializer;

use crate::{
    application::pagination::{JobCursor, PageRequest},
    application::{
        admin::jobs::AdminJobError,
        error::HttpError,
        repos::{JobQueryFilter, SettingsRepo},
    },
    domain::types::{JobState, JobType},
    infra::http::repo_error_to_http,
    presentation::{admin::views as admin_views, views::render_template_response},
};

use super::{AdminState, shared::flash_message};

const JOB_LIST_LIMIT: u32 = 100;

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub(super) struct AdminJobsQuery {
    status: Option<String>,
    state: Option<String>,
    job_type: Option<String>,
    search: Option<String>,
    cursor: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub(super) struct AdminJobActionForm {
    state: Option<String>,
    job_type: Option<String>,
    search: Option<String>,
    cursor: Option<String>,
}

fn normalize_filter_value(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.to_string())
}

fn parse_job_state_key(
    value: Option<&str>,
    source: &'static str,
) -> Result<Option<JobState>, HttpError> {
    let Some(raw) = normalize_filter_value(value) else {
        return Ok(None);
    };

    let lowered = raw.to_lowercase();
    let state = match lowered.as_str() {
        "pending" => JobState::Pending,
        "scheduled" => JobState::Scheduled,
        "running" => JobState::Running,
        "done" => JobState::Done,
        "failed" => JobState::Failed,
        "killed" => JobState::Killed,
        _ => {
            return Err(HttpError::new(
                source,
                StatusCode::BAD_REQUEST,
                "Invalid filter",
                format!("Unknown job state `{raw}`"),
            ));
        }
    };

    Ok(Some(state))
}

fn parse_job_type_key(
    value: Option<&str>,
    source: &'static str,
) -> Result<Option<JobType>, HttpError> {
    let Some(raw) = normalize_filter_value(value) else {
        return Ok(None);
    };

    match JobType::try_from(raw.as_str()) {
        Ok(job_type) => Ok(Some(job_type)),
        Err(_) => Err(HttpError::new(
            source,
            StatusCode::BAD_REQUEST,
            "Invalid filter",
            format!("Unknown job type `{raw}`"),
        )),
    }
}

fn job_state_key(state: JobState) -> &'static str {
    match state {
        JobState::Pending => "pending",
        JobState::Scheduled => "scheduled",
        JobState::Running => "running",
        JobState::Done => "done",
        JobState::Failed => "failed",
        JobState::Killed => "killed",
    }
}

fn job_type_label(job_type: JobType) -> &'static str {
    match job_type {
        JobType::RenderPost => "Render Post",
        JobType::RenderPostSections => "Render Post Sections",
        JobType::RenderPostSection => "Render Post Section",
        JobType::RenderPage => "Render Page",
        JobType::RenderSummary => "Render Summary",
        JobType::PublishPost => "Publish Post",
        JobType::PublishPage => "Publish Page",
        JobType::InvalidateCache => "Invalidate Cache",
    }
}

fn build_job_filter_from_query(
    query: &AdminJobsQuery,
    source: &'static str,
) -> Result<JobQueryFilter, HttpError> {
    let state = parse_job_state_key(query.state.as_deref(), source)?;
    let job_type = parse_job_type_key(query.job_type.as_deref(), source)?;
    let search = normalize_filter_value(query.search.as_deref());

    Ok(JobQueryFilter {
        state,
        job_type,
        search,
    })
}

fn build_job_filter_from_form(
    form: &AdminJobActionForm,
    source: &'static str,
) -> Result<JobQueryFilter, HttpError> {
    let state = parse_job_state_key(form.state.as_deref(), source)?;
    let job_type = parse_job_type_key(form.job_type.as_deref(), source)?;
    let search = normalize_filter_value(form.search.as_deref());

    Ok(JobQueryFilter {
        state,
        job_type,
        search,
    })
}

fn build_job_filter_query(filter: &JobQueryFilter) -> String {
    let mut serializer = Serializer::new(String::new());

    if let Some(state) = filter.state {
        serializer.append_pair("state", job_state_key(state));
    }

    if let Some(job_type) = filter.job_type {
        serializer.append_pair("job_type", job_type.as_str());
    }

    if let Some(search) = filter.search.as_deref() {
        serializer.append_pair("search", search);
    }

    serializer.finish()
}

fn decode_job_cursor(
    value: Option<&str>,
    source: &'static str,
) -> Result<Option<JobCursor>, HttpError> {
    match normalize_filter_value(value) {
        Some(raw) => JobCursor::decode(&raw).map(Some).map_err(|err| {
            HttpError::new(
                source,
                StatusCode::BAD_REQUEST,
                "Invalid cursor",
                err.to_string(),
            )
        }),
        None => Ok(None),
    }
}

fn append_query(base: &str, query: &str) -> String {
    if query.is_empty() {
        base.to_string()
    } else if base.contains('?') {
        format!("{base}&{query}")
    } else {
        format!("{base}?{query}")
    }
}

fn merge_filter_and_cursor(filter_query: &str, cursor: Option<&str>) -> String {
    match (filter_query.is_empty(), cursor) {
        (true, None) => String::new(),
        (true, Some(cursor)) => format!("cursor={cursor}"),
        (false, None) => filter_query.to_string(),
        (false, Some(cursor)) => format!("{filter_query}&cursor={cursor}"),
    }
}

fn job_state_options(selected: Option<JobState>) -> Vec<admin_views::AdminJobFilterOption> {
    let mut options = Vec::with_capacity(1 + 6);
    options.push(admin_views::AdminJobFilterOption {
        value: String::new(),
        label: "All states".to_string(),
        selected: selected.is_none(),
    });

    for state in [
        JobState::Pending,
        JobState::Scheduled,
        JobState::Running,
        JobState::Done,
        JobState::Failed,
        JobState::Killed,
    ] {
        options.push(admin_views::AdminJobFilterOption {
            value: job_state_key(state).to_string(),
            label: state.as_str().to_string(),
            selected: selected == Some(state),
        });
    }

    options
}

fn job_type_options(selected: Option<JobType>) -> Vec<admin_views::AdminJobFilterOption> {
    let mut options = Vec::with_capacity(1 + 6);
    options.push(admin_views::AdminJobFilterOption {
        value: String::new(),
        label: "All types".to_string(),
        selected: selected.is_none(),
    });

    for job_type in [
        JobType::RenderPost,
        JobType::RenderPage,
        JobType::RenderSummary,
        JobType::PublishPost,
        JobType::PublishPage,
        JobType::InvalidateCache,
    ] {
        options.push(admin_views::AdminJobFilterOption {
            value: job_type.as_str().to_string(),
            label: job_type_label(job_type).to_string(),
            selected: selected == Some(job_type),
        });
    }

    options
}

pub(super) async fn admin_jobs(
    State(state): State<AdminState>,
    Query(query): Query<AdminJobsQuery>,
) -> Response {
    let chrome = match state.chrome.load("/jobs").await {
        Ok(chrome) => chrome,
        Err(err) => return err.into_response(),
    };

    let flash = flash_from_status(query.status.as_deref());

    let filter = match build_job_filter_from_query(&query, "infra::http::admin_jobs") {
        Ok(filter) => filter,
        Err(err) => return err.into_response(),
    };

    let cursor = match decode_job_cursor(query.cursor.as_deref(), "infra::http::admin_jobs") {
        Ok(cursor) => cursor,
        Err(err) => return err.into_response(),
    };

    let page_request = PageRequest::new(JOB_LIST_LIMIT, cursor);

    let timezone = match state.db.load_site_settings().await {
        Ok(settings) => settings.timezone,
        Err(err) => return admin_job_error("infra::http::admin_jobs", err.into()).into_response(),
    };

    let page = match state.jobs.list_jobs(&filter, page_request).await {
        Ok(list) => list,
        Err(err) => return admin_job_error("infra::http::admin_jobs", err).into_response(),
    };

    let filter_query = build_job_filter_query(&filter);
    let filter_search = filter.search.clone();
    let state = filter.state;
    let job_type = filter.job_type;
    let filter_state_key = state.map(|s| job_state_key(s).to_string());
    let filter_job_type_key = job_type.map(|t| t.as_str().to_string());
    let cursor_param = query.cursor.as_deref();

    let state_options = job_state_options(state);
    let type_options = job_type_options(job_type);

    let next_cursor = page.next_cursor.clone();

    let rows = page
        .items
        .into_iter()
        .map(|job| {
            let detail_query = merge_filter_and_cursor(&filter_query, cursor_param);
            admin_views::AdminJobRowView {
                id: job.id.clone(),
                job_type: job.job_type.as_str().to_string(),
                status: job.state.as_str().to_string(),
                scheduled_at: Some(admin_views::format_timestamp(job.run_at, timezone)),
                created_at: job
                    .done_at
                    .or(job.lock_at)
                    .map(|time| admin_views::format_timestamp(time, timezone))
                    .unwrap_or_else(|| admin_views::format_timestamp(job.run_at, timezone)),
                error_text: job.last_error,
                detail_href: append_query(&format!("/jobs/{}", job.id), &detail_query),
                retry_action: format!("/jobs/{}/retry", job.id),
                cancel_action: format!("/jobs/{}/cancel", job.id),
                can_retry: is_retryable(job.state),
                can_cancel: is_cancellable(job.state),
            }
        })
        .collect();

    let content = admin_views::AdminJobListView {
        heading: "Jobs".to_string(),
        jobs: rows,
        state_options,
        type_options,
        filter_search,
        filter_state: filter_state_key,
        filter_job_type: filter_job_type_key,
        filter_query,
        current_cursor: query.cursor.clone(),
        next_cursor,
        flash,
    };

    let view = admin_views::AdminLayout::new(chrome, content);
    render_template_response(admin_views::AdminJobsTemplate { view }, StatusCode::OK)
}

pub(super) async fn admin_job_detail(
    State(state): State<AdminState>,
    Path(id): Path<String>,
    Query(query): Query<AdminJobsQuery>,
) -> Response {
    let chrome = match state.chrome.load("/jobs").await {
        Ok(chrome) => chrome,
        Err(err) => return err.into_response(),
    };

    let filter = match build_job_filter_from_query(&query, "infra::http::admin_job_detail") {
        Ok(filter) => filter,
        Err(err) => return err.into_response(),
    };

    if let Err(err) = decode_job_cursor(query.cursor.as_deref(), "infra::http::admin_job_detail") {
        return err.into_response();
    }

    let filter_query = build_job_filter_query(&filter);
    let preserved_query = merge_filter_and_cursor(&filter_query, query.cursor.as_deref());

    let job = match state.jobs.load_job(&id).await {
        Ok(job) => job,
        Err(AdminJobError::NotFound) => {
            let href = append_query("/jobs?status=not_found", &preserved_query);
            return Redirect::to(&href).into_response();
        }
        Err(err) => return admin_job_error("infra::http::admin_job_detail", err).into_response(),
    };

    let payload_pretty = to_string_pretty(&job.payload).unwrap_or_else(|_| job.payload.to_string());

    let timezone = match state.db.load_site_settings().await {
        Ok(settings) => settings.timezone,
        Err(err) => {
            return admin_job_error("infra::http::admin_job_detail", err.into()).into_response();
        }
    };

    let filter_search = filter.search.clone();
    let state = filter.state;
    let job_type = filter.job_type;
    let filter_state_key = state.map(|s| job_state_key(s).to_string());
    let filter_job_type_key = job_type.map(|t| t.as_str().to_string());
    let back_href = append_query("/jobs", &preserved_query);
    let retry_action = format!("/jobs/{}/retry", job.id);
    let cancel_action = format!("/jobs/{}/cancel", job.id);

    let content = admin_views::AdminJobDetailView {
        id: job.id.clone(),
        job_type: job.job_type.as_str().to_string(),
        status: job.state.as_str().to_string(),
        attempts: job.attempts,
        max_attempts: job.max_attempts,
        run_at: admin_views::format_timestamp(job.run_at, timezone),
        lock_at: job
            .lock_at
            .map(|time| admin_views::format_timestamp(time, timezone)),
        lock_by: job.lock_by,
        done_at: job
            .done_at
            .map(|time| admin_views::format_timestamp(time, timezone)),
        last_error: job.last_error,
        priority: job.priority,
        payload_pretty,
        retry_action,
        cancel_action,
        back_href,
        filter_state: filter_state_key,
        filter_job_type: filter_job_type_key,
        filter_search,
        filter_cursor: query.cursor.clone(),
        can_retry: is_retryable(job.state),
        can_cancel: is_cancellable(job.state),
        flash: flash_from_status(query.status.as_deref()),
        filter_query,
    };

    let view = admin_views::AdminLayout::new(chrome, content);
    render_template_response(admin_views::AdminJobDetailTemplate { view }, StatusCode::OK)
}

pub(super) async fn admin_job_retry(
    State(state): State<AdminState>,
    Path(id): Path<String>,
    Form(form): Form<AdminJobActionForm>,
) -> Response {
    let actor = "admin";
    let filter = match build_job_filter_from_form(&form, "infra::http::admin_job_retry") {
        Ok(filter) => filter,
        Err(err) => return err.into_response(),
    };

    let filter_query = build_job_filter_query(&filter);
    if let Err(err) = decode_job_cursor(form.cursor.as_deref(), "infra::http::admin_job_retry") {
        return err.into_response();
    }
    let preserved_query = merge_filter_and_cursor(&filter_query, form.cursor.as_deref());

    match state.jobs.retry_job(actor, &id).await {
        Ok(_) => {
            let mut target = format!("/jobs/{id}?status=retried");
            if !preserved_query.is_empty() {
                target.push('&');
                target.push_str(&preserved_query);
            }
            Redirect::to(&target).into_response()
        }
        Err(AdminJobError::NotFound) => {
            let href = append_query("/jobs?status=not_found", &preserved_query);
            Redirect::to(&href).into_response()
        }
        Err(err) => admin_job_error("infra::http::admin_job_retry", err).into_response(),
    }
}

pub(super) async fn admin_job_cancel(
    State(state): State<AdminState>,
    Path(id): Path<String>,
    Form(form): Form<AdminJobActionForm>,
) -> Response {
    let actor = "admin";
    let filter = match build_job_filter_from_form(&form, "infra::http::admin_job_cancel") {
        Ok(filter) => filter,
        Err(err) => return err.into_response(),
    };

    let filter_query = build_job_filter_query(&filter);
    if let Err(err) = decode_job_cursor(form.cursor.as_deref(), "infra::http::admin_job_cancel") {
        return err.into_response();
    }
    let preserved_query = merge_filter_and_cursor(&filter_query, form.cursor.as_deref());

    match state
        .jobs
        .cancel_job(actor, &id, Some("Cancelled via admin"))
        .await
    {
        Ok(_) => {
            let mut target = format!("/jobs/{id}?status=cancelled");
            if !preserved_query.is_empty() {
                target.push('&');
                target.push_str(&preserved_query);
            }
            Redirect::to(&target).into_response()
        }
        Err(AdminJobError::NotFound) => {
            let href = append_query("/jobs?status=not_found", &preserved_query);
            Redirect::to(&href).into_response()
        }
        Err(err) => admin_job_error("infra::http::admin_job_cancel", err).into_response(),
    }
}

fn admin_job_error(source: &'static str, err: AdminJobError) -> HttpError {
    match err {
        AdminJobError::NotFound => HttpError::new(
            source,
            StatusCode::NOT_FOUND,
            "Job not found",
            "The requested job does not exist",
        ),
        AdminJobError::Repo(repo) => repo_error_to_http(source, repo),
    }
}

fn flash_from_status(status: Option<&str>) -> Option<admin_views::AdminFlashMessage> {
    status.map(|value| match value {
        "retried" => flash_message("success", "Job requeued"),
        "cancelled" => flash_message("success", "Job cancelled"),
        "not_found" => flash_message("error", "Job not found"),
        _ => flash_message("info", "Operation completed"),
    })
}

fn is_retryable(state: JobState) -> bool {
    matches!(state, JobState::Failed | JobState::Killed | JobState::Done)
}

fn is_cancellable(state: JobState) -> bool {
    matches!(
        state,
        JobState::Pending | JobState::Scheduled | JobState::Running
    )
}
