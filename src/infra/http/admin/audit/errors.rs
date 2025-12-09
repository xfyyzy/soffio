//! Error conversion for audit admin handlers.

use crate::application::{error::HttpError, repos::RepoError};
use crate::infra::http::repo_error_to_http;

pub(super) fn admin_audit_error(source: &'static str, err: RepoError) -> HttpError {
    repo_error_to_http(source, err)
}
