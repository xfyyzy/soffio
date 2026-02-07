use std::path::PathBuf;

use clap::{Args, Parser, Subcommand, ValueHint, builder::BoolishValueParser};

/// Command-line arguments for the Soffio binary.
#[derive(Debug, Parser)]
#[command(name = "soffio", version, about = "Soffio Blog server")]
pub struct CliArgs {
    /// Optional path to a configuration file.
    #[arg(long = "config-file", env = "SOFFIO_CONFIG_FILE", value_name = "PATH")]
    pub config_file: Option<PathBuf>,

    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Debug, Subcommand, Clone)]
pub enum Command {
    /// Run the Soffio HTTP services.
    Serve(Box<ServeArgs>),
    /// Re-render all stored posts and pages.
    #[command(name = "renderall")]
    RenderAll(RenderAllArgs),
    /// Export site content and configuration to a TOML archive.
    #[command(name = "export")]
    ExportSite(ExportArgs),
    /// Import site content and configuration from a TOML archive.
    #[command(name = "import")]
    ImportSite(ImportArgs),
    /// Migration utilities.
    #[command(name = "migrations")]
    Migrations(MigrationsArgs),
}

#[derive(Debug, Args, Clone)]
pub struct RenderAllArgs {
    #[command(flatten)]
    pub overrides: RenderAllOverrides,

    /// Render posts; when neither --posts nor --pages is supplied, both are rendered.
    #[arg(long, action = clap::ArgAction::SetTrue)]
    pub posts: bool,

    /// Render pages; when neither --posts nor --pages is supplied, both are rendered.
    #[arg(long, action = clap::ArgAction::SetTrue)]
    pub pages: bool,

    /// Maximum number of concurrent render tasks.
    #[arg(long, default_value_t = 4, value_parser = clap::value_parser!(usize))]
    pub concurrency: usize,
}

#[derive(Debug, Args, Default, Clone)]
pub struct DatabaseOverride {
    /// Override the database connection URL.
    #[arg(long = "database-url", value_name = "URL")]
    pub database_url: Option<String>,
}

#[derive(Debug, Args, Default, Clone)]
pub struct ServeArgs {
    #[command(flatten)]
    pub overrides: ServeOverrides,
}

#[derive(Debug, Args, Default, Clone)]
pub struct RenderOverrides {
    /// Override the Mermaid CLI executable path used for diagram rendering.
    #[arg(long = "render-mermaid-cli-path", value_name = "PATH")]
    pub mermaid_cli_path: Option<PathBuf>,

    /// Override the directory used to cache rendered Mermaid diagrams.
    #[arg(long = "render-mermaid-cache-dir", value_name = "PATH")]
    pub mermaid_cache_dir: Option<PathBuf>,
}

#[derive(Debug, Args, Default, Clone)]
pub struct ServeOverrides {
    #[command(flatten)]
    pub render: RenderOverrides,

    /// Override the public listener host.
    #[arg(long = "server-host", value_name = "HOST")]
    pub server_host: Option<String>,

    /// Override the administrative listener host.
    #[arg(long = "server-admin-host", value_name = "HOST")]
    pub server_admin_host: Option<String>,

    /// Override the public listener port.
    #[arg(long = "server-public-port", value_name = "PORT")]
    pub public_port: Option<u16>,

    /// Override the administrative listener port.
    #[arg(long = "server-admin-port", value_name = "PORT")]
    pub admin_port: Option<u16>,

    /// Override the graceful shutdown timeout.
    #[arg(long = "server-graceful-shutdown-seconds", value_name = "SECONDS")]
    pub server_graceful_shutdown_seconds: Option<u64>,

    /// Override the base log level (trace|debug|info|warn|error).
    #[arg(long = "log-level", value_name = "LEVEL")]
    pub log_level: Option<String>,

    /// Toggle JSON logging.
    #[arg(
        long = "log-json",
        value_name = "BOOL",
        value_parser = BoolishValueParser::new()
    )]
    pub log_json: Option<bool>,

    /// Override the database connection URL.
    #[arg(long = "database-url", value_name = "URL")]
    pub database_url: Option<String>,

    /// Override the HTTP database pool size.
    #[arg(long = "database-http-max-connections", value_name = "COUNT")]
    pub database_http_max_connections: Option<u32>,

    /// Override the jobs database pool size.
    #[arg(long = "database-jobs-max-connections", value_name = "COUNT")]
    pub database_jobs_max_connections: Option<u32>,

    /// Override the render-post worker concurrency.
    #[arg(long = "jobs-render-post-concurrency", value_name = "COUNT")]
    pub jobs_render_post_concurrency: Option<u32>,

    /// Override the render-summary worker concurrency.
    #[arg(long = "jobs-render-summary-concurrency", value_name = "COUNT")]
    pub jobs_render_summary_concurrency: Option<u32>,

    /// Override the render-page worker concurrency.
    #[arg(long = "jobs-render-page-concurrency", value_name = "COUNT")]
    pub jobs_render_page_concurrency: Option<u32>,

    /// Override the publish-post worker concurrency.
    #[arg(long = "jobs-publish-post-concurrency", value_name = "COUNT")]
    pub jobs_publish_post_concurrency: Option<u32>,

    /// Override the publish-page worker concurrency.
    #[arg(long = "jobs-publish-page-concurrency", value_name = "COUNT")]
    pub jobs_publish_page_concurrency: Option<u32>,

    /// Override the uploads directory.
    #[arg(long = "uploads-directory", value_name = "PATH")]
    pub uploads_directory: Option<PathBuf>,

    /// Override the maximum request size for uploads in bytes.
    #[arg(long = "uploads-max-request-bytes", value_name = "BYTES")]
    pub uploads_max_request_bytes: Option<u64>,

    /// Override the rate limit window size.
    #[arg(long = "rate-limit-window-seconds", value_name = "SECONDS")]
    pub rate_limit_window_seconds: Option<u64>,

    /// Override the rate limit request ceiling.
    #[arg(long = "rate-limit-max-requests", value_name = "COUNT")]
    pub rate_limit_max_requests: Option<u64>,

    /// Override the API rate limit window size.
    #[arg(long = "api-rate-limit-window-seconds", value_name = "SECONDS")]
    pub api_rate_limit_window_seconds: Option<u64>,

    /// Override the API rate limit request ceiling.
    #[arg(long = "api-rate-limit-max-requests", value_name = "COUNT")]
    pub api_rate_limit_max_requests: Option<u64>,

    /// Override the background scheduler cadence.
    #[arg(long = "scheduler-cadence-seconds", value_name = "SECONDS")]
    pub scheduler_cadence_seconds: Option<u64>,

    /// Enable L0 object cache.
    #[arg(
        long = "cache-enable-l0-cache",
        value_name = "BOOL",
        value_parser = BoolishValueParser::new()
    )]
    pub cache_enable_l0_cache: Option<bool>,

    /// Enable L1 response cache.
    #[arg(
        long = "cache-enable-l1-cache",
        value_name = "BOOL",
        value_parser = BoolishValueParser::new()
    )]
    pub cache_enable_l1_cache: Option<bool>,

    /// Override the L0 post limit.
    #[arg(long = "cache-l0-post-limit", value_name = "COUNT")]
    pub cache_l0_post_limit: Option<usize>,

    /// Override the L0 page limit.
    #[arg(long = "cache-l0-page-limit", value_name = "COUNT")]
    pub cache_l0_page_limit: Option<usize>,

    /// Override the L0 API key limit.
    #[arg(long = "cache-l0-api-key-limit", value_name = "COUNT")]
    pub cache_l0_api_key_limit: Option<usize>,

    /// Override the L0 post list limit.
    #[arg(long = "cache-l0-post-list-limit", value_name = "COUNT")]
    pub cache_l0_post_list_limit: Option<usize>,

    /// Override the L1 response limit.
    #[arg(long = "cache-l1-response-limit", value_name = "COUNT")]
    pub cache_l1_response_limit: Option<usize>,

    /// Override the L1 response body limit in bytes.
    #[arg(long = "cache-l1-response-body-limit-bytes", value_name = "BYTES")]
    pub cache_l1_response_body_limit_bytes: Option<usize>,

    /// Override the cache auto-consume interval in milliseconds.
    #[arg(long = "cache-auto-consume-interval-ms", value_name = "MS")]
    pub cache_auto_consume_interval_ms: Option<u64>,

    /// Override the cache consume batch limit.
    #[arg(long = "cache-consume-batch-limit", value_name = "COUNT")]
    pub cache_consume_batch_limit: Option<usize>,

    /// Override the maximum cache event queue length.
    #[arg(long = "cache-max-event-queue-len", value_name = "COUNT")]
    pub cache_max_event_queue_len: Option<usize>,
}

#[derive(Debug, Args, Default, Clone)]
pub struct RenderAllOverrides {
    #[command(flatten)]
    pub database: DatabaseOverride,

    #[command(flatten)]
    pub render: RenderOverrides,
}

#[derive(Debug, Args, Clone)]
pub struct ExportArgs {
    #[command(flatten)]
    pub database: DatabaseOverride,

    /// Path to the export file to write.
    #[arg(value_name = "FILE", value_hint = ValueHint::FilePath)]
    pub file: PathBuf,
}

#[derive(Debug, Args, Clone)]
pub struct ImportArgs {
    #[command(flatten)]
    pub database: DatabaseOverride,

    /// Path to the archive to import.
    #[arg(value_name = "FILE", value_hint = ValueHint::FilePath)]
    pub file: PathBuf,
}

#[derive(Debug, Args, Clone)]
pub struct MigrationsArgs {
    #[command(subcommand)]
    pub command: MigrationsCommand,
}

#[derive(Debug, Subcommand, Clone)]
pub enum MigrationsCommand {
    /// Reconcile archive migration entries with the live database.
    #[command(name = "reconcile")]
    Reconcile(MigrationsReconcileArgs),
}

#[derive(Debug, Args, Clone)]
pub struct MigrationsReconcileArgs {
    #[command(flatten)]
    pub database: DatabaseOverride,

    /// Archive TOML file whose [[migrations.entries]] will be updated.
    #[arg(value_name = "ARCHIVE", value_hint = ValueHint::FilePath)]
    pub file: PathBuf,
}
