use thiserror::Error;

#[derive(Debug, Error)]
pub enum DomainError {
    #[error("domain entity `{entity}` not found")]
    NotFound { entity: &'static str },
    #[error("domain validation failed: {message}")]
    Validation { message: String },
    #[error("domain invariant violated: {message}")]
    Invariant { message: String },
}

impl DomainError {
    pub fn not_found(entity: &'static str) -> Self {
        Self::NotFound { entity }
    }

    pub fn validation(message: impl Into<String>) -> Self {
        Self::Validation {
            message: message.into(),
        }
    }

    pub fn invariant(message: impl Into<String>) -> Self {
        Self::Invariant {
            message: message.into(),
        }
    }
}
