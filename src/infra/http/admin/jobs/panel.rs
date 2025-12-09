//! Panel building for job list.

use askama::Template;

use crate::{
    application::{
        admin::jobs::AdminJobError,
        error::HttpError,
        pagination::{JobCursor, PageRequest},
        repos::{JobQueryFilter, SettingsRepo},
    },
    domain::{entities::JobRecord, types::JobState},
    infra::http::admin::{AdminState, pagination::CursorState, shared::template_render_http_error},
    presentation::admin::views as admin_views,
};

use super::status::{
    job_type_key, job_type_label, job_type_options, state_key, state_label, status_filters,
};

/// Build the complete job list view for rendering.
pub(super) async fn build_job_list_view(
    state: &AdminState,
    status: Option<JobState>,
    filter: &JobQueryFilter,
    cursor: Option<JobCursor>,
) -> Result<admin_views::AdminJobListView, AdminJobError> {
    let settings = state.db.load_site_settings().await?;
    let admin_page_size = settings.admin_page_size.clamp(1, 100).max(1) as u32;

    let page_request = PageRequest::new(admin_page_size, cursor);

    // Build base filter for counts (without state filter, but with search for accurate counts)
    let base_filter = JobQueryFilter {
        state: None,
        job_type: filter.job_type,
        search: filter.search.clone(),
    };

    // Build filter for the main list query
    let list_filter = JobQueryFilter {
        state: status,
        job_type: filter.job_type,
        search: filter.search.clone(),
    };

    // Execute all queries in parallel using service methods
    let (page, status_counts, type_counts) = tokio::try_join!(
        state.jobs.list_jobs(&list_filter, page_request),
        state.jobs.status_counts(&base_filter),
        state.jobs.type_counts(&base_filter),
    )?;

    let jobs = page
        .items
        .into_iter()
        .map(|job| {
            let actions = job_actions_for_state(job.state);
            admin_views::AdminJobRowView {
                id: job.id.clone(),
                detail_href: format!("/jobs/{}", job.id),
                job_type_key: job_type_key(job.job_type).to_string(),
                job_type_label: job_type_label(job.job_type).to_string(),
                state_key: state_key(job.state).to_string(),
                state_label: state_label(job.state).to_string(),
                attempts: format!("{}/{}", job.attempts, job.max_attempts),
                run_at: admin_views::format_timestamp(job.run_at, settings.timezone),
                done_at: job
                    .done_at
                    .map(|time| admin_views::format_timestamp(time, settings.timezone)),
                last_error: job.last_error,
                actions,
            }
        })
        .collect();

    let filters = status_filters(&status_counts, status);

    Ok(admin_views::AdminJobListView {
        heading: "Jobs".to_string(),
        filters,
        jobs,
        filter_job_type: filter.job_type.map(|jt| job_type_key(jt).to_string()),
        filter_search: filter.search.clone(),
        filter_query: String::new(),
        active_status_key: status.map(|s| state_key(s).to_string()),
        job_type_options: job_type_options(&type_counts),
        job_type_filter_enabled: true,
        next_cursor: page.next_cursor,
        cursor_param: None,
        trail: None,
        previous_page_state: None,
        next_page_state: None,
        panel_action: "/jobs/panel".to_string(),
        row_action_prefix: "/jobs".to_string(),
        custom_hidden_fields: build_job_hidden_fields(filter),
    })
}

fn build_job_hidden_fields(filter: &JobQueryFilter) -> Vec<admin_views::AdminHiddenField> {
    let mut fields = Vec::new();
    if let Some(jt) = filter.job_type {
        fields.push(admin_views::AdminHiddenField::new(
            "job_type",
            job_type_key(jt),
        ));
    }
    fields
}

/// Build the job detail view for a single job.
pub(super) async fn build_job_detail_view(
    state: &AdminState,
    job: JobRecord,
) -> Result<admin_views::AdminJobDetailView, AdminJobError> {
    let settings = state.db.load_site_settings().await?;

    let mut fields = Vec::new();

    // ID field
    fields.push(admin_views::AdminJobDetailField {
        label: "ID".to_string(),
        value: job.id.clone(),
        is_badge: false,
        badge_status: None,
        is_multiline: false,
    });

    // Type field with badge
    fields.push(admin_views::AdminJobDetailField {
        label: "Type".to_string(),
        value: job_type_label(job.job_type).to_string(),
        is_badge: true,
        badge_status: Some(job_type_key(job.job_type).to_string()),
        is_multiline: false,
    });

    // State field with badge
    fields.push(admin_views::AdminJobDetailField {
        label: "State".to_string(),
        value: state_label(job.state).to_string(),
        is_badge: true,
        badge_status: Some(state_key(job.state).to_string()),
        is_multiline: false,
    });

    // Attempts field
    fields.push(admin_views::AdminJobDetailField {
        label: "Attempts".to_string(),
        value: format!("{}/{}", job.attempts, job.max_attempts),
        is_badge: false,
        badge_status: None,
        is_multiline: false,
    });

    // Priority field
    fields.push(admin_views::AdminJobDetailField {
        label: "Priority".to_string(),
        value: job.priority.to_string(),
        is_badge: false,
        badge_status: None,
        is_multiline: false,
    });

    // Run at field
    fields.push(admin_views::AdminJobDetailField {
        label: "Run At".to_string(),
        value: admin_views::format_timestamp(job.run_at, settings.timezone),
        is_badge: false,
        badge_status: None,
        is_multiline: false,
    });

    // Done at field (if applicable)
    if let Some(done_at) = job.done_at {
        fields.push(admin_views::AdminJobDetailField {
            label: "Done At".to_string(),
            value: admin_views::format_timestamp(done_at, settings.timezone),
            is_badge: false,
            badge_status: None,
            is_multiline: false,
        });
    }

    // Payload field (JSON formatted)
    let payload_str =
        serde_json::to_string_pretty(&job.payload).unwrap_or_else(|_| job.payload.to_string());
    fields.push(admin_views::AdminJobDetailField {
        label: "Payload".to_string(),
        value: payload_str,
        is_badge: false,
        badge_status: None,
        is_multiline: true,
    });

    // Last error field (if any)
    if let Some(last_error) = &job.last_error {
        fields.push(admin_views::AdminJobDetailField {
            label: "Last Error".to_string(),
            value: last_error.clone(),
            is_badge: false,
            badge_status: None,
            is_multiline: true,
        });
    }

    Ok(admin_views::AdminJobDetailView {
        heading: format!("Job: {}", job.id),
        fields,
    })
}

/// Determine available actions based on job state.
fn job_actions_for_state(state: JobState) -> Vec<admin_views::AdminJobRowActionView> {
    match state {
        JobState::Pending | JobState::Scheduled => vec![admin_views::AdminJobRowActionView {
            value: "cancel",
            label: "Cancel",
            is_danger: true,
        }],
        JobState::Failed | JobState::Killed => vec![admin_views::AdminJobRowActionView {
            value: "retry",
            label: "Retry",
            is_danger: false,
        }],
        _ => vec![],
    }
}

/// Render job panel HTML from view.
pub(super) fn render_job_panel_html(
    content: &admin_views::AdminJobListView,
    template_source: &'static str,
) -> Result<String, HttpError> {
    let template = admin_views::AdminJobsPanelTemplate {
        content: content.clone(),
    };

    template.render().map_err(|err| {
        template_render_http_error(template_source, "Template rendering failed", err)
    })
}

/// Apply pagination links to job list view.
pub(super) fn apply_pagination_links(
    content: &mut admin_views::AdminJobListView,
    cursor_state: &CursorState,
) {
    use crate::infra::http::admin::pagination;

    content.cursor_param = cursor_state.current_token();
    content.trail = pagination::join_cursor_history(cursor_state.history_tokens());

    let mut previous_history = cursor_state.clone_history();
    let previous_token = previous_history.pop();

    content.previous_page_state = previous_token.map(|token| {
        let previous_cursor_value = pagination::decode_cursor_token(&token);
        let previous_trail = pagination::join_cursor_history(&previous_history);
        admin_views::AdminPostPaginationState {
            cursor: previous_cursor_value,
            trail: previous_trail,
        }
    });

    if let Some(next_cursor) = content.next_cursor.clone() {
        let mut next_history = cursor_state.clone_history();
        next_history.push(pagination::encode_cursor_token(
            cursor_state.current_token_ref(),
        ));
        let next_trail = pagination::join_cursor_history(&next_history);
        content.next_page_state = Some(admin_views::AdminPostPaginationState {
            cursor: Some(next_cursor),
            trail: next_trail,
        });
    } else {
        content.next_page_state = None;
    }
}
