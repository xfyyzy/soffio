use serde::Serialize;
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum AdminTagError {
    #[error("{0}")]
    ConstraintViolation(&'static str),
    #[error("tag is referenced by {count} posts")]
    InUse { count: u64 },
    #[error(transparent)]
    Repo(#[from] crate::application::repos::RepoError),
}

#[derive(Debug, Clone)]
pub struct CreateTagCommand {
    pub name: String,
    pub description: Option<String>,
    pub pinned: bool,
}

#[derive(Debug, Clone)]
pub struct UpdateTagCommand {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub pinned: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct AdminTagStatusCounts {
    pub total: u64,
    pub pinned: u64,
    pub unpinned: u64,
}

pub(super) fn ensure_non_empty(value: &str, field: &'static str) -> Result<(), AdminTagError> {
    if value.trim().is_empty() {
        return Err(AdminTagError::ConstraintViolation(field));
    }
    Ok(())
}

pub(super) fn normalize_optional_text(value: Option<String>) -> Option<String> {
    value.and_then(|value| {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    })
}
