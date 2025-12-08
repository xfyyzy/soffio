//! Settings admin module.
//!
//! This module handles the admin interface for site settings with split responsibilities:
//! - `handlers`: HTTP handler functions
//! - `forms`: Form struct definitions and validation
//! - `views`: View building functions
//! - `validators`: Input validation helpers
//! - `errors`: Error handling utilities

mod errors;
mod forms;
mod handlers;
mod validators;
mod views;

pub(super) use handlers::{admin_settings, admin_settings_edit, admin_settings_update};
