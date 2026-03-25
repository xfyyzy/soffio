use askama::Template;
use axum::{
    extract::{Form, Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Redirect, Response},
};
use uuid::Uuid;

use crate::{
    application::{
        admin::tags::{CreateTagCommand, UpdateTagCommand},
        pagination::TagCursor,
        repos::TagQueryFilter,
    },
    domain::entities::TagRecord,
    infra::http::admin::{
        AdminState,
        selectors::{PANEL, TAGS_PANEL},
        shared::{
            AdminPostQuery, EditorSuccessRender, Toast, blank_to_none_opt, datastar_replace,
            push_toasts, stream_editor_success, template_render_http_error,
        },
    },
    presentation::{admin::views as admin_views, views::render_template_response},
};

use crate::infra::http::admin::pagination::CursorState;

use super::{
    editor::{build_new_tag_view, build_tag_edit_view},
    errors::admin_tag_error,
    forms::{AdminTagDeleteForm, AdminTagForm, AdminTagPanelForm, AdminTagPinForm},
    panel::{apply_pagination_links, build_tag_list_view, render_tag_panel_html},
    status::{parse_tag_status, tag_status_label},
};

#[path = "handlers/editing.rs"]
mod editing;
#[path = "handlers/listing.rs"]
mod listing;
#[path = "handlers/mutations.rs"]
mod mutations;
#[path = "handlers/shared.rs"]
mod shared;

pub(crate) use editing::{admin_tag_create, admin_tag_edit, admin_tag_new, admin_tag_update};
pub(crate) use listing::{admin_tags, admin_tags_panel};
pub(crate) use mutations::{admin_tag_delete, admin_tag_pin, admin_tag_unpin};
