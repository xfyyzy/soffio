use thiserror::Error;

#[derive(Debug, Error)]
pub enum InfraError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("database error: {message}")]
    Database { message: String },
    #[error("telemetry initialization failed: {0}")]
    Telemetry(String),
    #[error("configuration error: {message}")]
    Configuration { message: String },
}

impl InfraError {
    pub fn database(message: impl Into<String>) -> Self {
        Self::Database {
            message: message.into(),
        }
    }

    pub fn configuration(message: impl Into<String>) -> Self {
        Self::Configuration {
            message: message.into(),
        }
    }

    pub fn telemetry(message: impl Into<String>) -> Self {
        Self::Telemetry(message.into())
    }
}
