//! Post admin module.
//!
//! This module handles the admin interface for posts with split responsibilities:
//! - `crud`: List, create, edit, update, delete operations
//! - `bulk`: Bulk actions on multiple posts
//! - `status_actions`: Publish, draft, archive status changes
//! - `pin`: Pin/unpin operations
//! - `tags`: Tag selection and toggle
//! - `response`: Shared response helper functions
//! - `pagination`: Cursor pagination helpers
//! - `utils`: Common utility functions
//! - `panel`: Panel building for post list
//! - `sections`: Editor view building
//! - `forms`: Form struct definitions
//! - `status`: Status filtering and parsing
//! - `errors`: Error conversion utilities

mod bulk;
mod crud;
mod errors;
mod forms;
mod pagination;
mod panel;
mod pin;
mod response;
mod sections;
mod status;
mod status_actions;
mod tags;
mod utils;

// Re-export all public handlers
pub(super) use bulk::admin_posts_bulk_action;
pub(super) use crud::{
    admin_post_create, admin_post_delete, admin_post_edit, admin_post_new, admin_post_update,
    admin_posts, admin_posts_panel,
};
pub(super) use pin::{admin_post_pin, admin_post_unpin};
pub(super) use status_actions::{admin_post_archive, admin_post_move_to_draft, admin_post_publish};
pub(super) use tags::{admin_post_tags_toggle, admin_post_tags_toggle_new};
