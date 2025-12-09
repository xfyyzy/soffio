use askama::{Error as AskamaError, Template};
use axum::response::{IntoResponse, Response};
use datastar::prelude::ElementPatchMode;
use serde::Deserialize;
use std::time::Duration;
use uuid::Uuid;

use super::selectors::{PANEL, TOAST_STACK};
use crate::{
    application::{error::HttpError, stream::StreamBuilder},
    presentation::{admin::views as admin_views, views::TemplateRenderError},
};

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub(super) struct AdminPostQuery {
    pub(super) status: Option<String>,
    pub(super) cursor: Option<String>,
    pub(super) trail: Option<String>,
    pub(super) search: Option<String>,
    pub(super) tag: Option<String>,
    pub(super) month: Option<String>,
}

pub(super) fn blank_to_none_opt(value: Option<String>) -> Option<String> {
    value.and_then(|v| {
        let trimmed = v.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    })
}

#[derive(Clone)]
pub(super) struct Toast {
    pub id: Uuid,
    pub kind: ToastKind,
    pub text: String,
    pub ttl: Duration,
}

#[derive(Clone, Copy)]
pub(super) enum ToastKind {
    Success,
    Error,
}

impl ToastKind {
    fn as_variant(self) -> &'static str {
        match self {
            ToastKind::Success => "success",
            ToastKind::Error => "error",
        }
    }
}

const DEFAULT_TOAST_TTL: Duration = Duration::from_millis(6000);

impl Toast {
    pub fn success(text: impl Into<String>) -> Self {
        Self::success_with_ttl(text, DEFAULT_TOAST_TTL)
    }

    pub fn error(text: impl Into<String>) -> Self {
        Self::error_with_ttl(text, DEFAULT_TOAST_TTL)
    }

    pub fn success_with_ttl(text: impl Into<String>, ttl: Duration) -> Self {
        Self {
            id: Uuid::new_v4(),
            kind: ToastKind::Success,
            text: text.into(),
            ttl,
        }
    }

    pub fn error_with_ttl(text: impl Into<String>, ttl: Duration) -> Self {
        Self {
            id: Uuid::new_v4(),
            kind: ToastKind::Error,
            text: text.into(),
            ttl,
        }
    }
}

pub(super) fn push_toasts(stream: &mut StreamBuilder, toasts: &[Toast]) -> Result<(), HttpError> {
    let view_items = toasts
        .iter()
        .map(|toast| admin_views::AdminToastItem {
            id: toast.id.to_string(),
            kind: toast.kind.as_variant(),
            text: toast.text.clone(),
            ttl_ms: toast.ttl.as_millis() as u64,
        })
        .collect::<Vec<_>>();

    let template = admin_views::AdminToastStackTemplate { toasts: view_items };

    let html = template.render().map_err(|err| {
        template_render_http_error(
            "infra::http::admin::push_toasts",
            "Template rendering failed",
            err,
        )
    })?;

    stream.push_patch(html, TOAST_STACK, ElementPatchMode::Replace);
    Ok(())
}

pub(super) fn datastar_replace(selector: &str, html: String) -> StreamBuilder {
    let mut stream = StreamBuilder::new();
    stream.push_patch(html, selector, ElementPatchMode::Replace);
    stream
}

pub(super) struct EditorSuccessRender<'a> {
    pub editor_html: String,
    pub panel_html: String,
    pub panel_selector: &'static str,
    pub toasts: &'a [Toast],
    pub history_path: Option<String>,
}

pub(super) fn stream_editor_success(params: EditorSuccessRender<'_>) -> Response {
    let EditorSuccessRender {
        editor_html,
        panel_html,
        panel_selector,
        toasts,
        history_path,
    } = params;

    let mut stream = StreamBuilder::new();
    stream.push_patch(editor_html, PANEL, ElementPatchMode::Replace);
    stream.push_patch(panel_html, panel_selector, ElementPatchMode::Replace);

    if !toasts.is_empty()
        && let Err(err) = push_toasts(&mut stream, toasts)
    {
        return err.into_response();
    }

    if let Some(path) = history_path {
        stream.push_script(format!(
            "window.history.replaceState(null, '', '{}');",
            path
        ));
    }

    stream.into_response()
}

pub(super) fn template_render_http_error(
    source: &'static str,
    message: &'static str,
    err: AskamaError,
) -> HttpError {
    HttpError::from(TemplateRenderError::new(source, message, err))
}
