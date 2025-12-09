//! Audit admin module.
//!
//! Read-only admin interface for viewing audit logs.

mod errors;
mod forms;
mod handlers;
mod panel;
mod status;

pub(super) use handlers::{admin_audit, admin_audit_panel};
