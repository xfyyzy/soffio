//! Navigation admin HTTP handlers.

use axum::{
    extract::{Form, Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Redirect, Response},
};
use uuid::Uuid;

use crate::{
    application::{
        admin::navigation::{CreateNavigationItemCommand, UpdateNavigationItemCommand},
        pagination::NavigationCursor,
        repos::NavigationQueryFilter,
    },
    domain::{entities::NavigationItemRecord, types::NavigationDestinationType},
    infra::http::admin::{
        AdminState,
        pagination::CursorState,
        selectors::{NAVIGATION_PANEL, PANEL},
        shared::{
            EditorSuccessRender, Toast, blank_to_none_opt, datastar_replace, push_toasts,
            stream_editor_success,
        },
    },
    presentation::{admin::views as admin_views, views::render_template_response},
};

use super::editor::{build_navigation_editor_view, render_navigation_editor_panel};
use super::forms::{
    AdminNavigationDeleteForm, AdminNavigationForm, AdminNavigationPanelForm, AdminNavigationQuery,
    AdminNavigationVisibilityForm,
};
use super::panel::{
    admin_navigation_error, apply_navigation_pagination_links, build_navigation_list_view,
    build_navigation_panel_html, render_navigation_panel_html,
};
use super::status::{
    NavigationListStatus, build_navigation_filter, parse_navigation_status, parse_navigation_type,
    parse_optional_uuid,
};

#[path = "handlers/editing.rs"]
mod editing;
#[path = "handlers/listing.rs"]
mod listing;
#[path = "handlers/mutations.rs"]
mod mutations;

pub(crate) use editing::{
    admin_navigation_create, admin_navigation_destination_preview,
    admin_navigation_destination_preview_for_item, admin_navigation_edit, admin_navigation_new,
    admin_navigation_update,
};
pub(crate) use listing::{admin_navigation, admin_navigation_panel};
pub(crate) use mutations::{admin_navigation_delete, admin_navigation_toggle_visibility};
