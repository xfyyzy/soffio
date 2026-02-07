use std::sync::Once;

use metrics::{Unit, describe_counter, describe_gauge, describe_histogram};
use tracing_error::ErrorLayer;
use tracing_subscriber::{
    EnvFilter, fmt,
    layer::{Layer, SubscriberExt},
    util::SubscriberInitExt,
};

use crate::config::{LogFormat, LoggingSettings};

use super::error::InfraError;

static METRIC_DESCRIPTIONS: Once = Once::new();

/// Install a global tracing subscriber using the provided logging settings.
pub fn init(logging: &LoggingSettings) -> Result<(), InfraError> {
    describe_metrics();

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

fn describe_metrics() {
    METRIC_DESCRIPTIONS.call_once(|| {
        describe_counter!(
            "soffio_cache_l0_hit_total",
            Unit::Count,
            "Total number of L0 cache hits."
        );
        describe_counter!(
            "soffio_cache_l0_miss_total",
            Unit::Count,
            "Total number of L0 cache misses."
        );
        describe_counter!(
            "soffio_cache_l0_evict_total",
            Unit::Count,
            "Total number of L0 cache evictions due to capacity."
        );
        describe_counter!(
            "soffio_cache_l1_hit_total",
            Unit::Count,
            "Total number of L1 response-cache hits."
        );
        describe_counter!(
            "soffio_cache_l1_miss_total",
            Unit::Count,
            "Total number of L1 response-cache misses."
        );
        describe_counter!(
            "soffio_cache_l1_evict_total",
            Unit::Count,
            "Total number of L1 response-cache evictions due to capacity."
        );
        describe_gauge!(
            "soffio_cache_event_queue_len",
            Unit::Count,
            "Current number of pending cache events in the queue."
        );
        describe_counter!(
            "soffio_cache_event_dropped_total",
            Unit::Count,
            "Total number of cache events dropped due to queue overflow."
        );
        describe_histogram!(
            "soffio_cache_consume_ms",
            Unit::Milliseconds,
            "Cache consumption latency in milliseconds."
        );
        describe_histogram!(
            "soffio_cache_warm_ms",
            Unit::Milliseconds,
            "Cache warm phase latency in milliseconds."
        );
    });
}
