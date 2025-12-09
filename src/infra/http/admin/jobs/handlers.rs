//! HTTP handlers for jobs admin - list and actions.

use askama::Template;
use axum::{
    extract::{Form, Path, State},
    response::{IntoResponse, Response},
};

use crate::{
    application::{pagination::JobCursor, repos::JobQueryFilter},
    infra::http::admin::{
        AdminState,
        pagination::CursorState,
        selectors::JOBS_PANEL,
        shared::{Toast, datastar_replace, push_toasts},
    },
    presentation::admin::views as admin_views,
};

use super::{
    errors::admin_job_error,
    forms::{AdminJobActionForm, AdminJobsPanelForm},
    panel::{
        apply_pagination_links, build_job_detail_view, build_job_list_view, render_job_panel_html,
    },
    status::{parse_job_state, parse_job_type},
};

/// GET /jobs - Render jobs list page.
pub(crate) async fn admin_jobs(State(state): State<AdminState>) -> Response {
    let filter = JobQueryFilter {
        state: None,
        job_type: None,
        search: None,
    };

    let mut content = match build_job_list_view(&state, None, &filter, None).await {
        Ok(content) => content,
        Err(err) => {
            return admin_job_error("infra::http::admin::jobs::admin_jobs", err).into_response();
        }
    };

    let cursor_state = CursorState::default();
    apply_pagination_links(&mut content, &cursor_state);

    let chrome = match state.chrome.load("/jobs").await {
        Ok(chrome) => chrome,
        Err(err) => return err.into_response(),
    };
    let view = admin_views::AdminLayout::new(chrome, content);
    let template = admin_views::AdminJobsTemplate { view };

    match template.render() {
        Ok(html) => axum::response::Html(html).into_response(),
        Err(err) => {
            tracing::error!(error = %err, "Failed to render jobs template");
            axum::http::StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

/// GET /jobs/{id} - Render single job detail page.
pub(crate) async fn admin_job_detail(
    State(state): State<AdminState>,
    Path(id): Path<String>,
) -> Response {
    let job = match state.jobs.load_job(&id).await {
        Ok(job) => job,
        Err(err) => {
            return admin_job_error("infra::http::admin::jobs::admin_job_detail", err)
                .into_response();
        }
    };

    let content = match build_job_detail_view(&state, job).await {
        Ok(content) => content,
        Err(err) => {
            return admin_job_error("infra::http::admin::jobs::admin_job_detail", err)
                .into_response();
        }
    };

    let chrome = match state.chrome.load("/jobs").await {
        Ok(chrome) => chrome,
        Err(err) => return err.into_response(),
    };
    let view = admin_views::AdminLayout::new(chrome, content);
    let template = admin_views::AdminJobDetailTemplate { view };

    match template.render() {
        Ok(html) => axum::response::Html(html).into_response(),
        Err(err) => {
            tracing::error!(error = %err, "Failed to render job detail template");
            axum::http::StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

/// POST /jobs/panel - AJAX panel refresh via datastar SSE.
pub(crate) async fn admin_jobs_panel(
    State(state): State<AdminState>,
    Form(form): Form<AdminJobsPanelForm>,
) -> Response {
    let status = match parse_job_state(form.status.as_deref()) {
        Ok(status) => status,
        Err(err) => return err.into_response(),
    };

    let (job_type, search) = if form.clear.is_some() {
        (None, None)
    } else {
        let jt = match parse_job_type(form.job_type.as_deref()) {
            Ok(jt) => jt,
            Err(err) => return err.into_response(),
        };
        let s = form.search.clone().filter(|s| !s.is_empty());
        (jt, s)
    };

    let filter = JobQueryFilter {
        state: status,
        job_type,
        search,
    };

    let cursor_state = CursorState::new(form.cursor.clone(), form.trail.clone());
    let cursor = match cursor_state.decode_with(
        JobCursor::decode,
        "infra::http::admin::jobs::admin_jobs_panel",
    ) {
        Ok(cursor) => cursor,
        Err(err) => return err.into_response(),
    };

    let mut content = match build_job_list_view(&state, status, &filter, cursor).await {
        Ok(content) => content,
        Err(err) => {
            return admin_job_error("infra::http::admin::jobs::admin_jobs_panel", err)
                .into_response();
        }
    };

    apply_pagination_links(&mut content, &cursor_state);

    match render_job_panel_html(&content, "infra::http::admin::jobs::admin_jobs_panel") {
        Ok(html) => datastar_replace(JOBS_PANEL, html).into_response(),
        Err(err) => err.into_response(),
    }
}

/// POST /jobs/{id}/retry - Retry a failed/killed job.
pub(crate) async fn admin_job_retry(
    State(state): State<AdminState>,
    Path(id): Path<String>,
    Form(form): Form<AdminJobActionForm>,
) -> Response {
    match state.jobs.retry_job("admin", &id).await {
        Ok(_job) => {
            respond_with_panel_and_toast(
                &state,
                &form,
                Toast::success(format!("Job {} queued for retry", id)),
            )
            .await
        }
        Err(err) => {
            admin_job_error("infra::http::admin::jobs::admin_job_retry", err).into_response()
        }
    }
}

/// POST /jobs/{id}/cancel - Cancel a pending/scheduled job.
pub(crate) async fn admin_job_cancel(
    State(state): State<AdminState>,
    Path(id): Path<String>,
    Form(form): Form<AdminJobActionForm>,
) -> Response {
    match state
        .jobs
        .cancel_job("admin", &id, Some("Cancelled by admin"))
        .await
    {
        Ok(_job) => {
            respond_with_panel_and_toast(
                &state,
                &form,
                Toast::success(format!("Job {} cancelled", id)),
            )
            .await
        }
        Err(err) => {
            admin_job_error("infra::http::admin::jobs::admin_job_cancel", err).into_response()
        }
    }
}

/// Helper to respond with refreshed panel and toast message.
async fn respond_with_panel_and_toast(
    state: &AdminState,
    form: &AdminJobActionForm,
    toast: Toast,
) -> Response {
    let status = match parse_job_state(form.status_filter.as_deref()) {
        Ok(status) => status,
        Err(err) => return err.into_response(),
    };

    let job_type = match parse_job_type(form.filter_job_type.as_deref()) {
        Ok(jt) => jt,
        Err(err) => return err.into_response(),
    };

    let search = form.filter_search.clone().filter(|s| !s.is_empty());

    let filter = JobQueryFilter {
        state: status,
        job_type,
        search,
    };

    let cursor_state = CursorState::new(form.cursor.clone(), form.trail.clone());
    let cursor = match cursor_state.decode_with(
        JobCursor::decode,
        "infra::http::admin::jobs::respond_with_panel_and_toast",
    ) {
        Ok(cursor) => cursor,
        Err(err) => return err.into_response(),
    };

    let mut content = match build_job_list_view(state, status, &filter, cursor).await {
        Ok(content) => content,
        Err(err) => {
            return admin_job_error(
                "infra::http::admin::jobs::respond_with_panel_and_toast",
                err,
            )
            .into_response();
        }
    };

    apply_pagination_links(&mut content, &cursor_state);

    match render_job_panel_html(
        &content,
        "infra::http::admin::jobs::respond_with_panel_and_toast",
    ) {
        Ok(html) => {
            let mut stream = datastar_replace(JOBS_PANEL, html);
            if let Err(err) = push_toasts(&mut stream, &[toast]) {
                return err.into_response();
            }
            stream.into_response()
        }
        Err(err) => err.into_response(),
    }
}
