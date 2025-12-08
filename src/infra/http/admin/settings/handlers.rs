//! HTTP handlers for settings admin.

use askama::Template;
use axum::{
    extract::{Form, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};

use crate::application::admin::settings::AdminSettingsError;
use crate::application::error::HttpError;
use crate::presentation::{admin::views as admin_views, views::render_template_response};

use super::super::{
    AdminState,
    selectors::PANEL,
    shared::{Toast, datastar_replace, push_toasts, template_render_http_error},
};

use super::errors::admin_settings_error;
use super::forms::AdminSettingsForm;
use super::views::{edit_view_from_record, summary_view_from_record};

const SOURCE_BASE: &str = "infra::http::admin_settings";

pub(crate) async fn admin_settings(State(state): State<AdminState>) -> Response {
    let chrome = match state.chrome.load("/settings").await {
        Ok(chrome) => chrome,
        Err(err) => return err.into_response(),
    };

    let settings = match state.settings.load().await {
        Ok(settings) => settings,
        Err(err) => return admin_settings_error(SOURCE_BASE, err).into_response(),
    };

    let content = summary_view_from_record(&settings);
    let view = admin_views::AdminLayout::new(chrome, content);
    render_template_response(admin_views::AdminSettingsTemplate { view }, StatusCode::OK)
}

pub(crate) async fn admin_settings_edit(State(state): State<AdminState>) -> Response {
    let chrome = match state.chrome.load("/settings/edit").await {
        Ok(chrome) => chrome,
        Err(err) => return err.into_response(),
    };

    let settings = match state.settings.load().await {
        Ok(settings) => settings,
        Err(err) => {
            return admin_settings_error("infra::http::admin_settings_edit", err).into_response();
        }
    };

    let content = edit_view_from_record(&settings);
    let view = admin_views::AdminLayout::new(chrome, content);
    render_template_response(
        admin_views::AdminSettingsEditTemplate { view },
        StatusCode::OK,
    )
}

pub(crate) async fn admin_settings_update(
    State(state): State<AdminState>,
    Form(form): Form<AdminSettingsForm>,
) -> Response {
    let original = match state.settings.load().await {
        Ok(settings) => settings,
        Err(err) => {
            return admin_settings_error("infra::http::admin_settings_update_load", err)
                .into_response();
        }
    };

    let original_updated = admin_views::format_timestamp(original.updated_at, original.timezone);

    let command = match form.to_command() {
        Ok(command) => command,
        Err(err) => {
            let content = form.to_edit_view(original_updated);
            return render_editor_with_toast(
                content,
                &[Toast::error(err.to_string())],
                "infra::http::admin_settings_update",
            )
            .into_response();
        }
    };

    let actor = "admin";
    match state.settings.update(actor, command).await {
        Ok(updated) => {
            let content = edit_view_from_record(&updated);
            render_editor_with_toast(
                content,
                &[Toast::success("Site settings updated successfully")],
                "infra::http::admin_settings_update",
            )
            .into_response()
        }
        Err(err) => match err {
            AdminSettingsError::ConstraintViolation(field) => {
                let content = form.to_edit_view(original_updated);
                render_editor_with_toast(
                    content,
                    &[Toast::error(format!("Field `{field}` cannot be empty"))],
                    "infra::http::admin_settings_update",
                )
                .into_response()
            }
            AdminSettingsError::Repo(repo) => admin_settings_error(
                "infra::http::admin_settings_update",
                AdminSettingsError::Repo(repo),
            )
            .into_response(),
        },
    }
}

fn render_editor_with_toast(
    content: admin_views::AdminSettingsEditView,
    toasts: &[Toast],
    template_source: &'static str,
) -> Response {
    let panel_html = match render_settings_editor_panel(&content, template_source) {
        Ok(html) => html,
        Err(err) => return err.into_response(),
    };

    let mut stream = datastar_replace(PANEL, panel_html);
    if let Err(err) = push_toasts(&mut stream, toasts) {
        return err.into_response();
    }

    stream.into_response()
}

fn render_settings_editor_panel(
    content: &admin_views::AdminSettingsEditView,
    template_source: &'static str,
) -> Result<String, HttpError> {
    let template = admin_views::AdminSettingsEditPanelTemplate {
        content: content.clone(),
    };

    template.render().map_err(|err| {
        template_render_http_error(template_source, "Template rendering failed", err)
    })
}
