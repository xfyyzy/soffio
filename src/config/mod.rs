//! Configuration layer: typed settings with layered precedence (file → env → CLI).

use std::{
    net::SocketAddr,
    num::{NonZeroU32, NonZeroU64},
    path::PathBuf,
    str::FromStr,
    time::Duration,
};

use clap::{Args, Parser, Subcommand, ValueHint, builder::BoolishValueParser};
use config::{Config, Environment, File};
use serde::Deserialize;
use thiserror::Error;
use tracing::level_filters::LevelFilter;

const DEFAULT_CONFIG_BASENAME: &str = "config/default";
const LOCAL_CONFIG_BASENAME: &str = "soffio";
const DEFAULT_HOST: &str = "127.0.0.1";
const DEFAULT_ADMIN_HOST: &str = "127.0.0.1";
const DEFAULT_PUBLIC_PORT: u16 = 3000;
const DEFAULT_ADMIN_PORT: u16 = 3001;
const DEFAULT_GRACEFUL_SHUTDOWN_SECS: u64 = 30;
const DEFAULT_UPLOAD_DIR: &str = "uploads";
const DEFAULT_RATE_LIMIT_WINDOW_SECS: u64 = 60;
const DEFAULT_RATE_LIMIT_MAX_REQUESTS: u64 = 180;
const DEFAULT_API_RATE_LIMIT_WINDOW_SECS: u64 = 60;
const DEFAULT_API_RATE_LIMIT_MAX_REQUESTS: u64 = 120;
const DEFAULT_SCHEDULER_CADENCE_SECS: u64 = 300;
const DEFAULT_UPLOAD_REQUEST_LIMIT_BYTES: u64 = 10 * 1024 * 1024;
const DEFAULT_DB_HTTP_MAX_CONNECTIONS: u32 = 8;
const DEFAULT_DB_JOBS_MAX_CONNECTIONS: u32 = 8;
const DEFAULT_JOB_RENDER_POST_CONCURRENCY: u32 = 2;
const DEFAULT_JOB_RENDER_SUMMARY_CONCURRENCY: u32 = 2;
const DEFAULT_JOB_RENDER_PAGE_CONCURRENCY: u32 = 1;
const DEFAULT_JOB_PUBLISH_POST_CONCURRENCY: u32 = 1;
const DEFAULT_JOB_PUBLISH_PAGE_CONCURRENCY: u32 = 1;
pub(crate) const DEFAULT_MERMAID_CLI_PATH: &str = "mmdc";
pub(crate) const DEFAULT_MERMAID_CACHE_DIR: &str = "/tmp/soffio-mermaid";

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

#[derive(Debug, Error)]
pub enum LoadError {
    #[error("failed to build configuration: {0}")]
    Build(#[from] config::ConfigError),
    #[error("invalid configuration for `{key}`: {reason}")]
    Invalid { key: &'static str, reason: String },
}

impl LoadError {
    fn invalid(key: &'static str, reason: impl Into<String>) -> Self {
        Self::Invalid {
            key,
            reason: reason.into(),
        }
    }
}

/// Load settings using the configured precedence (file → environment → CLI).
pub fn load(cli: &CliArgs) -> Result<Settings, LoadError> {
    let mut builder = Config::builder()
        .add_source(File::with_name(DEFAULT_CONFIG_BASENAME).required(false))
        .add_source(File::with_name(LOCAL_CONFIG_BASENAME).required(false));

    if let Some(path) = cli.config_file.as_ref() {
        builder = builder.add_source(File::from(path.as_path()).required(true));
    }

    builder = builder.add_source(Environment::with_prefix("SOFFIO").separator("__"));

    let mut raw: RawSettings = builder.build()?.try_deserialize()?;

    match cli.command.as_ref() {
        Some(Command::Serve(args)) => raw.apply_serve_overrides(&args.overrides),
        Some(Command::RenderAll(args)) => raw.apply_renderall_overrides(&args.overrides),
        Some(Command::ExportSite(args)) => raw.apply_database_override(&args.database),
        Some(Command::ImportSite(args)) => raw.apply_database_override(&args.database),
        Some(Command::Migrations(args)) => match &args.command {
            MigrationsCommand::Reconcile(reconcile) => {
                raw.apply_database_override(&reconcile.database)
            }
        },
        None => raw.apply_serve_overrides(&ServeOverrides::default()),
    }

    Settings::from_raw(raw)
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
struct RawSettings {
    server: RawServerSettings,
    logging: RawLoggingSettings,
    database: RawDatabaseSettings,
    jobs: RawJobsSettings,
    render: RawRenderSettings,
    uploads: RawUploadSettings,
    rate_limit: RawRateLimitSettings,
    api_rate_limit: RawApiRateLimitSettings,
    scheduler: RawSchedulerSettings,
}

impl RawSettings {
    fn apply_serve_overrides(&mut self, overrides: &ServeOverrides) {
        if let Some(host) = overrides.server_host.as_ref() {
            self.server.host = Some(host.clone());
        }
        if let Some(host) = overrides.server_admin_host.as_ref() {
            self.server.admin_host = Some(host.clone());
        }
        if let Some(port) = overrides.public_port {
            self.server.public_port = Some(port);
        }
        if let Some(port) = overrides.admin_port {
            self.server.admin_port = Some(port);
        }
        if let Some(seconds) = overrides.server_graceful_shutdown_seconds {
            self.server.graceful_shutdown_seconds = Some(seconds);
        }
        if let Some(level) = overrides.log_level.as_ref() {
            self.logging.level = Some(level.clone());
        }
        if let Some(json) = overrides.log_json {
            self.logging.json = Some(json);
        }
        if let Some(url) = overrides.database_url.as_ref() {
            self.database.url = Some(url.clone());
        }
        if let Some(max) = overrides.database_http_max_connections {
            self.database.http_max_connections = Some(max);
        }
        if let Some(max) = overrides.database_jobs_max_connections {
            self.database.jobs_max_connections = Some(max);
        }
        if let Some(directory) = overrides.uploads_directory.as_ref() {
            self.uploads.directory = Some(directory.clone());
        }
        if let Some(limit) = overrides.uploads_max_request_bytes {
            self.uploads.max_request_bytes = Some(limit);
        }
        if let Some(window) = overrides.rate_limit_window_seconds {
            self.rate_limit.window_seconds = Some(window);
        }
        if let Some(max) = overrides.rate_limit_max_requests {
            self.rate_limit.max_requests = Some(max);
        }
        if let Some(window) = overrides.api_rate_limit_window_seconds {
            self.api_rate_limit.window_seconds = Some(window);
        }
        if let Some(max) = overrides.api_rate_limit_max_requests {
            self.api_rate_limit.max_requests = Some(max);
        }
        if let Some(cadence) = overrides.scheduler_cadence_seconds {
            self.scheduler.cadence_seconds = Some(cadence);
        }
        if let Some(value) = overrides.jobs_render_post_concurrency {
            self.jobs.render_post_concurrency = Some(value);
        }
        if let Some(value) = overrides.jobs_render_summary_concurrency {
            self.jobs.render_summary_concurrency = Some(value);
        }
        if let Some(value) = overrides.jobs_render_page_concurrency {
            self.jobs.render_page_concurrency = Some(value);
        }
        if let Some(value) = overrides.jobs_publish_post_concurrency {
            self.jobs.publish_post_concurrency = Some(value);
        }
        if let Some(value) = overrides.jobs_publish_page_concurrency {
            self.jobs.publish_page_concurrency = Some(value);
        }

        self.apply_render_overrides(&overrides.render);
    }

    fn apply_renderall_overrides(&mut self, overrides: &RenderAllOverrides) {
        self.apply_database_override(&overrides.database);
        self.apply_render_overrides(&overrides.render);
    }

    fn apply_database_override(&mut self, overrides: &DatabaseOverride) {
        if let Some(url) = overrides.database_url.as_ref() {
            self.database.url = Some(url.clone());
        }
    }

    fn apply_render_overrides(&mut self, overrides: &RenderOverrides) {
        if let Some(path) = overrides.mermaid_cli_path.as_ref() {
            self.render.mermaid_cli_path = Some(path.clone());
        }
        if let Some(dir) = overrides.mermaid_cache_dir.as_ref() {
            self.render.mermaid_cache_dir = Some(dir.clone());
        }
    }
}

impl Settings {
    fn from_raw(raw: RawSettings) -> Result<Self, LoadError> {
        let RawSettings {
            server,
            logging,
            database,
            jobs,
            render,
            uploads,
            rate_limit,
            api_rate_limit,
            scheduler,
        } = raw;

        let server = build_server_settings(server)?;
        let logging = build_logging_settings(logging)?;
        let database = build_database_settings(database)?;
        let jobs = build_jobs_settings(jobs)?;
        let render = build_render_settings(render)?;
        let uploads = build_upload_settings(uploads)?;
        let rate_limit = build_rate_limit_settings(rate_limit)?;
        let api_rate_limit = build_api_rate_limit_settings(api_rate_limit)?;
        let scheduler = build_scheduler_settings(scheduler)?;

        Ok(Self {
            server,
            logging,
            database,
            jobs,
            render,
            uploads,
            rate_limit,
            api_rate_limit,
            scheduler,
        })
    }
}

fn build_server_settings(server: RawServerSettings) -> Result<ServerSettings, LoadError> {
    let host = server.host.unwrap_or_else(|| DEFAULT_HOST.to_string());
    let admin_host = server
        .admin_host
        .unwrap_or_else(|| DEFAULT_ADMIN_HOST.to_string());

    let public_port = server.public_port.unwrap_or(DEFAULT_PUBLIC_PORT);
    if public_port == 0 {
        return Err(LoadError::invalid(
            "server.public_port",
            "port must be greater than zero",
        ));
    }

    let admin_port = server.admin_port.unwrap_or(DEFAULT_ADMIN_PORT);
    if admin_port == 0 {
        return Err(LoadError::invalid(
            "server.admin_port",
            "port must be greater than zero",
        ));
    }

    let public_addr = parse_socket_addr(&host, public_port)
        .map_err(|reason| LoadError::invalid("server.public_addr", reason))?;
    let admin_addr = parse_socket_addr(&admin_host, admin_port)
        .map_err(|reason| LoadError::invalid("server.admin_addr", reason))?;

    let graceful_secs = server
        .graceful_shutdown_seconds
        .unwrap_or(DEFAULT_GRACEFUL_SHUTDOWN_SECS);
    if graceful_secs == 0 {
        return Err(LoadError::invalid(
            "server.graceful_shutdown_seconds",
            "must be greater than zero",
        ));
    }
    let graceful_shutdown = Duration::from_secs(graceful_secs);

    Ok(ServerSettings {
        public_addr,
        admin_addr,
        graceful_shutdown,
    })
}

fn build_logging_settings(logging: RawLoggingSettings) -> Result<LoggingSettings, LoadError> {
    let level = match logging.level {
        Some(level) => LevelFilter::from_str(level.as_str()).map_err(|err| {
            LoadError::invalid("logging.level", format!("failed to parse: {err}"))
        })?,
        None => LevelFilter::INFO,
    };

    let format = if logging.json.unwrap_or(false) {
        LogFormat::Json
    } else {
        LogFormat::Compact
    };

    Ok(LoggingSettings { level, format })
}

fn build_database_settings(database: RawDatabaseSettings) -> Result<DatabaseSettings, LoadError> {
    let url = database.url.and_then(|value| {
        let trimmed = value.trim();
        (!trimmed.is_empty()).then(|| trimmed.to_string())
    });

    let http_value = database
        .http_max_connections
        .unwrap_or(DEFAULT_DB_HTTP_MAX_CONNECTIONS);
    let jobs_value = database
        .jobs_max_connections
        .unwrap_or(DEFAULT_DB_JOBS_MAX_CONNECTIONS);

    let http_max_connections = non_zero_u32(http_value.into(), "database.http_max_connections")?;
    let jobs_max_connections = non_zero_u32(jobs_value.into(), "database.jobs_max_connections")?;

    Ok(DatabaseSettings {
        url,
        http_max_connections,
        jobs_max_connections,
    })
}

fn build_upload_settings(uploads: RawUploadSettings) -> Result<UploadSettings, LoadError> {
    let directory = uploads
        .directory
        .unwrap_or_else(|| PathBuf::from(DEFAULT_UPLOAD_DIR));

    let max_request_bytes_value = uploads
        .max_request_bytes
        .unwrap_or(DEFAULT_UPLOAD_REQUEST_LIMIT_BYTES);
    let max_request_bytes = NonZeroU64::new(max_request_bytes_value).ok_or_else(|| {
        LoadError::invalid("uploads.max_request_bytes", "must be greater than zero")
    })?;
    usize::try_from(max_request_bytes_value).map_err(|_| {
        LoadError::invalid(
            "uploads.max_request_bytes",
            "value exceeds supported range for usize",
        )
    })?;

    Ok(UploadSettings {
        directory,
        max_request_bytes,
    })
}

fn build_jobs_settings(jobs: RawJobsSettings) -> Result<JobsSettings, LoadError> {
    let render_post = jobs
        .render_post_concurrency
        .unwrap_or(DEFAULT_JOB_RENDER_POST_CONCURRENCY);
    let render_summary = jobs
        .render_summary_concurrency
        .unwrap_or(DEFAULT_JOB_RENDER_SUMMARY_CONCURRENCY);
    let render_page = jobs
        .render_page_concurrency
        .unwrap_or(DEFAULT_JOB_RENDER_PAGE_CONCURRENCY);
    let publish_post = jobs
        .publish_post_concurrency
        .unwrap_or(DEFAULT_JOB_PUBLISH_POST_CONCURRENCY);
    let publish_page = jobs
        .publish_page_concurrency
        .unwrap_or(DEFAULT_JOB_PUBLISH_PAGE_CONCURRENCY);

    Ok(JobsSettings {
        render_post_concurrency: non_zero_u32(render_post.into(), "jobs.render_post_concurrency")?,
        render_summary_concurrency: non_zero_u32(
            render_summary.into(),
            "jobs.render_summary_concurrency",
        )?,
        render_page_concurrency: non_zero_u32(render_page.into(), "jobs.render_page_concurrency")?,
        publish_post_concurrency: non_zero_u32(
            publish_post.into(),
            "jobs.publish_post_concurrency",
        )?,
        publish_page_concurrency: non_zero_u32(
            publish_page.into(),
            "jobs.publish_page_concurrency",
        )?,
    })
}

fn build_render_settings(render: RawRenderSettings) -> Result<RenderSettings, LoadError> {
    let cli_path = render
        .mermaid_cli_path
        .unwrap_or_else(|| PathBuf::from(DEFAULT_MERMAID_CLI_PATH));
    if cli_path.as_os_str().is_empty() {
        return Err(LoadError::invalid(
            "render.mermaid_cli_path",
            "path must not be empty",
        ));
    }

    let cache_dir = render
        .mermaid_cache_dir
        .unwrap_or_else(|| PathBuf::from(DEFAULT_MERMAID_CACHE_DIR));
    if cache_dir.as_os_str().is_empty() {
        return Err(LoadError::invalid(
            "render.mermaid_cache_dir",
            "path must not be empty",
        ));
    }

    Ok(RenderSettings {
        mermaid_cli_path: cli_path,
        mermaid_cache_dir: cache_dir,
    })
}

fn build_rate_limit_settings(
    rate_limit: RawRateLimitSettings,
) -> Result<RateLimitSettings, LoadError> {
    let window_seconds_val = rate_limit
        .window_seconds
        .unwrap_or(DEFAULT_RATE_LIMIT_WINDOW_SECS);
    let window_seconds = non_zero_u32(window_seconds_val, "rate_limit.window_seconds")?;

    let max_requests_val = rate_limit
        .max_requests
        .unwrap_or(DEFAULT_RATE_LIMIT_MAX_REQUESTS);
    let max_requests = non_zero_u32(max_requests_val, "rate_limit.max_requests")?;

    Ok(RateLimitSettings {
        window_seconds,
        max_requests,
    })
}

fn build_api_rate_limit_settings(
    rate_limit: RawApiRateLimitSettings,
) -> Result<ApiRateLimitSettings, LoadError> {
    let window_seconds_val = rate_limit
        .window_seconds
        .unwrap_or(DEFAULT_API_RATE_LIMIT_WINDOW_SECS);
    let window_seconds = non_zero_u32(window_seconds_val, "api_rate_limit.window_seconds")?;

    let max_requests_val = rate_limit
        .max_requests
        .unwrap_or(DEFAULT_API_RATE_LIMIT_MAX_REQUESTS);
    let max_requests = non_zero_u32(max_requests_val, "api_rate_limit.max_requests")?;

    Ok(ApiRateLimitSettings {
        window_seconds,
        max_requests,
    })
}

fn build_scheduler_settings(
    scheduler: RawSchedulerSettings,
) -> Result<SchedulerSettings, LoadError> {
    let cadence_seconds = scheduler
        .cadence_seconds
        .unwrap_or(DEFAULT_SCHEDULER_CADENCE_SECS);
    if cadence_seconds == 0 {
        return Err(LoadError::invalid(
            "scheduler.cadence_seconds",
            "must be greater than zero",
        ));
    }

    Ok(SchedulerSettings {
        cadence: Duration::from_secs(cadence_seconds),
    })
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
struct RawServerSettings {
    host: Option<String>,
    admin_host: Option<String>,
    public_port: Option<u16>,
    admin_port: Option<u16>,
    graceful_shutdown_seconds: Option<u64>,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
struct RawLoggingSettings {
    level: Option<String>,
    json: Option<bool>,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
struct RawDatabaseSettings {
    url: Option<String>,
    http_max_connections: Option<u32>,
    jobs_max_connections: Option<u32>,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
struct RawUploadSettings {
    directory: Option<PathBuf>,
    max_request_bytes: Option<u64>,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
struct RawJobsSettings {
    render_post_concurrency: Option<u32>,
    render_summary_concurrency: Option<u32>,
    render_page_concurrency: Option<u32>,
    publish_post_concurrency: Option<u32>,
    publish_page_concurrency: Option<u32>,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
struct RawRenderSettings {
    mermaid_cli_path: Option<PathBuf>,
    mermaid_cache_dir: Option<PathBuf>,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
struct RawRateLimitSettings {
    window_seconds: Option<u64>,
    max_requests: Option<u64>,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
struct RawApiRateLimitSettings {
    window_seconds: Option<u64>,
    max_requests: Option<u64>,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
struct RawSchedulerSettings {
    cadence_seconds: Option<u64>,
}

fn parse_socket_addr(host: &str, port: u16) -> Result<SocketAddr, String> {
    let candidate = format!("{host}:{port}");
    candidate
        .parse()
        .map_err(|err| format!("invalid address `{candidate}`: {err}"))
}

fn non_zero_u32(value: u64, key: &'static str) -> Result<NonZeroU32, LoadError> {
    if value == 0 {
        return Err(LoadError::invalid(key, "must be greater than zero"));
    }
    let value_u32: u32 = value
        .try_into()
        .map_err(|_| LoadError::invalid(key, "value exceeds supported range for u32"))?;
    NonZeroU32::new(value_u32).ok_or_else(|| LoadError::invalid(key, "must be greater than zero"))
}

/// Resolve configuration using the supplied CLI arguments, returning both for downstream use.
pub fn load_with_cli() -> Result<(CliArgs, Settings), LoadError> {
    let args = CliArgs::parse();
    let settings = load(&args)?;
    Ok((args, settings))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cli_overrides_take_highest_precedence() {
        let mut raw = RawSettings::default();
        raw.server.public_port = Some(4000);
        raw.logging.level = Some("info".to_string());

        let overrides = ServeOverrides {
            public_port: Some(4321),
            log_level: Some("debug".to_string()),
            ..Default::default()
        };

        raw.apply_serve_overrides(&overrides);
        let settings = Settings::from_raw(raw).expect("valid settings");

        assert_eq!(settings.server.public_addr.port(), 4321);
        assert_eq!(settings.logging.level, LevelFilter::DEBUG);
    }

    #[test]
    fn uploads_limit_defaults_to_10_mib() {
        let raw = RawSettings::default();
        let settings = Settings::from_raw(raw).expect("valid settings");
        assert_eq!(
            settings.uploads.max_request_bytes.get(),
            DEFAULT_UPLOAD_REQUEST_LIMIT_BYTES
        );
    }

    #[test]
    fn uploads_limit_can_be_overridden_via_cli() {
        let mut raw = RawSettings::default();
        let overrides = ServeOverrides {
            uploads_max_request_bytes: Some(1_572_864),
            ..Default::default()
        };

        raw.apply_serve_overrides(&overrides);
        let settings = Settings::from_raw(raw).expect("valid settings");
        assert_eq!(settings.uploads.max_request_bytes.get(), 1_572_864);
    }

    #[test]
    fn cli_json_logging_enforces_format() {
        let mut raw = RawSettings::default();
        let overrides = ServeOverrides {
            log_json: Some(true),
            ..Default::default()
        };

        raw.apply_serve_overrides(&overrides);
        let settings = Settings::from_raw(raw).expect("valid settings");

        assert!(matches!(settings.logging.format, LogFormat::Json));
    }

    #[test]
    fn default_to_serve_command() {
        let args = CliArgs::parse_from(["soffio"]);
        let command = args
            .command
            .unwrap_or(Command::Serve(Box::<ServeArgs>::default()));
        assert!(matches!(command, Command::Serve(_)));
    }

    #[test]
    fn parse_renderall_arguments() {
        let args = CliArgs::parse_from([
            "soffio",
            "renderall",
            "--database-url",
            "postgres://example",
            "--posts",
            "--concurrency",
            "8",
        ]);

        match args.command.expect("renderall command") {
            Command::RenderAll(render) => {
                assert_eq!(
                    render.overrides.database.database_url.as_deref(),
                    Some("postgres://example")
                );
                assert!(render.posts);
                assert!(!render.pages);
                assert_eq!(render.concurrency, 8);
            }
            _ => panic!("wrong command parsed"),
        }
    }

    #[test]
    fn parse_export_arguments() {
        let args = CliArgs::parse_from([
            "soffio",
            "export",
            "--database-url",
            "postgres://example",
            "/tmp/site.toml",
        ]);

        match args.command.expect("export command") {
            Command::ExportSite(export) => {
                assert_eq!(
                    export.database.database_url.as_deref(),
                    Some("postgres://example")
                );
                assert_eq!(export.file, std::path::Path::new("/tmp/site.toml"));
            }
            _ => panic!("wrong command parsed"),
        }
    }

    #[test]
    fn parse_import_arguments() {
        let args = CliArgs::parse_from([
            "soffio",
            "import",
            "--database-url",
            "postgres://example",
            "/tmp/site.toml",
        ]);

        match args.command.expect("import command") {
            Command::ImportSite(import) => {
                assert_eq!(
                    import.database.database_url.as_deref(),
                    Some("postgres://example")
                );
                assert_eq!(import.file, std::path::Path::new("/tmp/site.toml"));
            }
            _ => panic!("wrong command parsed"),
        }
    }

    #[test]
    fn parse_migrations_reconcile_arguments() {
        let args = CliArgs::parse_from([
            "soffio",
            "migrations",
            "reconcile",
            "--database-url",
            "postgres://example",
            "/tmp/archive.toml",
        ]);

        match args.command.expect("migrations command") {
            Command::Migrations(mig) => match mig.command {
                MigrationsCommand::Reconcile(rec) => {
                    assert_eq!(
                        rec.database.database_url.as_deref(),
                        Some("postgres://example")
                    );
                    assert_eq!(rec.file, std::path::Path::new("/tmp/archive.toml"));
                }
            },
            _ => panic!("wrong command parsed"),
        }
    }

    #[test]
    fn parse_serve_overrides() {
        let args = CliArgs::parse_from([
            "soffio",
            "serve",
            "--server-host",
            "0.0.0.0",
            "--database-url",
            "postgres://override",
        ]);

        match args.command.expect("serve command") {
            Command::Serve(serve) => {
                assert_eq!(serve.overrides.server_host.as_deref(), Some("0.0.0.0"));
                assert_eq!(
                    serve.overrides.database_url.as_deref(),
                    Some("postgres://override")
                );
            }
            _ => panic!("wrong command parsed"),
        }
    }
}
