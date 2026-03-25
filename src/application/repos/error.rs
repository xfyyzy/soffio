use thiserror::Error;

use crate::application::pagination::PaginationError;

#[derive(Debug, Error)]
pub enum RepoError {
    #[error("persistence error: {0}")]
    Persistence(String),
    #[error("duplicate record violates unique constraint `{constraint}`")]
    Duplicate { constraint: String },
    #[error("resource not found")]
    NotFound,
    #[error("invalid input: {message}")]
    InvalidInput { message: String },
    #[error("integrity error: {message}")]
    Integrity { message: String },
    #[error("database timeout")]
    Timeout,
    #[error(transparent)]
    Pagination(#[from] PaginationError),
}

impl RepoError {
    pub fn from_persistence(err: impl std::fmt::Display) -> Self {
        Self::Persistence(err.to_string())
    }
}
