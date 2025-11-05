use tracing_error::ErrorLayer;
use tracing_subscriber::{
    EnvFilter, fmt,
    layer::{Layer, SubscriberExt},
    util::SubscriberInitExt,
};

use crate::config::{LogFormat, LoggingSettings};

use super::error::InfraError;

/// Install a global tracing subscriber using the provided logging settings.
pub fn init(logging: &LoggingSettings) -> Result<(), InfraError> {
    let env_filter = EnvFilter::builder()
        .with_default_directive(logging.level.into())
        .from_env_lossy();

    let fmt_layer = match logging.format {
        LogFormat::Json => fmt::layer()
            .json()
            .with_current_span(true)
            .with_span_list(true)
            .with_target(true)
            .boxed(),
        LogFormat::Compact => fmt::layer().compact().with_target(true).boxed(),
    };

    tracing_subscriber::registry()
        .with(env_filter)
        .with(ErrorLayer::default())
        .with(fmt_layer)
        .try_init()
        .map_err(|err| {
            InfraError::telemetry(format!("failed to install tracing subscriber: {err}"))
        })
}
