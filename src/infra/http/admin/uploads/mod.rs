//! Upload admin module.
//!
//! This module handles the admin interface for file uploads with split responsibilities:
//! - `handlers`: HTTP handler functions
//! - `forms`: Form struct definitions  
//! - `errors`: Error handling utilities
//! - `panel`: Panel building and view construction
//! - `response`: Response helper functions
//! - `storage`: Upload storage processing logic
//! - `multipart`: Multipart payload parsing
//! - `queue`: Queue-related functions

mod errors;
mod forms;
mod handlers;
mod multipart;
mod panel;
mod queue;
mod response;
mod storage;

pub(super) use handlers::{
    admin_upload_delete, admin_upload_download, admin_upload_new, admin_upload_queue_preview,
    admin_upload_store, admin_uploads, admin_uploads_panel,
};
