use askama::Template;
use axum::{
    extract::{Form, Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use uuid::Uuid;

use crate::{
    application::{
        admin::pages::{CreatePageCommand, UpdatePageContentCommand, UpdatePageStatusCommand},
        error::HttpError,
        pagination::PageCursor,
        repos::{PageQueryFilter, SettingsRepo},
    },
    domain::{entities::PageRecord, types::PageStatus},
    infra::http::admin::{
        AdminState,
        pagination::{self, CursorState},
        selectors::{PAGES_PANEL, PANEL},
        shared::{
            AdminPostQuery, EditorSuccessRender, Toast, datastar_replace, push_toasts,
            stream_editor_success, template_render_http_error,
        },
    },
    presentation::{admin::views as admin_views, views::render_template_response},
};

use super::{
    editor::{build_new_page_editor_view, build_page_editor_view},
    errors::admin_page_error,
    forms::{AdminPageDeleteForm, AdminPageForm, AdminPagePanelForm, AdminPageStatusActionForm},
    panel::{build_page_list_view, build_page_panel_html, render_page_panel_html},
    status::{page_status_label, parse_page_status},
};

#[path = "handlers/editing.rs"]
mod editing;
#[path = "handlers/listing.rs"]
mod listing;
#[path = "handlers/mutations.rs"]
mod mutations;
#[path = "handlers/shared.rs"]
mod shared;

pub(crate) use editing::{admin_page_create, admin_page_edit, admin_page_new, admin_page_update};
pub(crate) use listing::{admin_page_panel, admin_pages};
pub(crate) use mutations::{
    admin_page_archive, admin_page_delete, admin_page_move_to_draft, admin_page_publish,
};
