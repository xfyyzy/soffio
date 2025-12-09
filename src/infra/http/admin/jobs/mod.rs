//! Jobs admin module.
//!
//! This module handles the admin interface for viewing and managing background jobs.

mod errors;
mod forms;
mod handlers;
mod panel;
mod status;

pub(super) use handlers::{
    admin_job_cancel, admin_job_detail, admin_job_retry, admin_jobs, admin_jobs_panel,
};
