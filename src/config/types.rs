use std::{
    net::SocketAddr,
    num::{NonZeroU32, NonZeroU64},
    path::PathBuf,
    time::Duration,
};

use thiserror::Error;
use tracing::level_filters::LevelFilter;

/// Fully-resolved deployment settings after precedence resolution and validation.
#[derive(Debug, Clone)]
pub struct Settings {
    pub server: ServerSettings,
    pub logging: LoggingSettings,
    pub database: DatabaseSettings,
    pub jobs: JobsSettings,
    pub render: RenderSettings,
    pub uploads: UploadSettings,
    pub rate_limit: RateLimitSettings,
    pub api_rate_limit: ApiRateLimitSettings,
    pub scheduler: SchedulerSettings,
    pub cache: CacheSettings,
}

#[derive(Debug, Clone)]
pub struct ServerSettings {
    pub public_addr: SocketAddr,
    pub admin_addr: SocketAddr,
    pub graceful_shutdown: Duration,
}

#[derive(Debug, Clone)]
pub struct LoggingSettings {
    pub level: LevelFilter,
    pub format: LogFormat,
}

#[derive(Debug, Clone, Copy)]
pub enum LogFormat {
    Json,
    Compact,
}

#[derive(Debug, Clone)]
pub struct DatabaseSettings {
    pub url: Option<String>,
    pub http_max_connections: NonZeroU32,
    pub jobs_max_connections: NonZeroU32,
}

#[derive(Debug, Clone)]
pub struct RenderSettings {
    pub mermaid_cli_path: PathBuf,
    pub mermaid_cache_dir: PathBuf,
}

#[derive(Debug, Clone)]
pub struct UploadSettings {
    pub directory: PathBuf,
    pub max_request_bytes: NonZeroU64,
}

#[derive(Debug, Clone)]
pub struct JobsSettings {
    pub render_post_concurrency: NonZeroU32,
    pub render_summary_concurrency: NonZeroU32,
    pub render_page_concurrency: NonZeroU32,
    pub publish_post_concurrency: NonZeroU32,
    pub publish_page_concurrency: NonZeroU32,
}

#[derive(Debug, Clone)]
pub struct RateLimitSettings {
    pub window_seconds: NonZeroU32,
    pub max_requests: NonZeroU32,
}

#[derive(Debug, Clone)]
pub struct ApiRateLimitSettings {
    pub window_seconds: NonZeroU32,
    pub max_requests: NonZeroU32,
}

#[derive(Debug, Clone)]
pub struct SchedulerSettings {
    pub cadence: Duration,
}

/// Fully-resolved cache settings.
#[derive(Debug, Clone)]
pub struct CacheSettings {
    /// Enable L0 object/query cache.
    pub enable_l0_cache: bool,
    /// Enable L1 response cache.
    pub enable_l1_cache: bool,
    /// Maximum posts in L0 KV cache.
    pub l0_post_limit: usize,
    /// Maximum pages in L0 KV cache.
    pub l0_page_limit: usize,
    /// Maximum API keys in L0 KV cache.
    pub l0_api_key_limit: usize,
    /// Maximum post list pages in L0 LRU cache.
    pub l0_post_list_limit: usize,
    /// Maximum HTTP responses in L1 cache.
    pub l1_response_limit: usize,
    /// Maximum HTTP response body size in bytes for L1 cache.
    pub l1_response_body_limit_bytes: usize,
    /// Auto-consume interval (ms) for eventual consistency.
    pub auto_consume_interval_ms: u64,
    /// Maximum events per consumption batch.
    pub consume_batch_limit: usize,
    /// Maximum queue length for cache events.
    pub max_event_queue_len: usize,
}

#[derive(Debug, Error)]
pub enum LoadError {
    #[error("failed to build configuration: {0}")]
    Build(#[from] config::ConfigError),
    #[error("invalid configuration for `{key}`: {reason}")]
    Invalid { key: &'static str, reason: String },
}

impl LoadError {
    pub(super) fn invalid(key: &'static str, reason: impl Into<String>) -> Self {
        Self::Invalid {
            key,
            reason: reason.into(),
        }
    }
}
