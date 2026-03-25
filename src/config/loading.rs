use std::{
    net::SocketAddr,
    num::{NonZeroU32, NonZeroU64},
    path::PathBuf,
    str::FromStr,
    time::Duration,
};

use clap::Parser;
use config::{Config, Environment, File};
use serde::Deserialize;
use tracing::level_filters::LevelFilter;

use super::cli::{CliArgs, Command, MigrationsCommand, ServeOverrides};
use super::defaults::{
    DEFAULT_ADMIN_HOST, DEFAULT_ADMIN_PORT, DEFAULT_API_RATE_LIMIT_MAX_REQUESTS,
    DEFAULT_API_RATE_LIMIT_WINDOW_SECS, DEFAULT_CACHE_AUTO_CONSUME_INTERVAL_MS,
    DEFAULT_CACHE_CONSUME_BATCH_LIMIT, DEFAULT_CACHE_L0_API_KEY_LIMIT, DEFAULT_CACHE_L0_PAGE_LIMIT,
    DEFAULT_CACHE_L0_POST_LIMIT, DEFAULT_CACHE_L0_POST_LIST_LIMIT,
    DEFAULT_CACHE_L1_RESPONSE_BODY_LIMIT_BYTES, DEFAULT_CACHE_L1_RESPONSE_LIMIT,
    DEFAULT_CACHE_MAX_EVENT_QUEUE_LEN, DEFAULT_CONFIG_BASENAME, DEFAULT_DB_HTTP_MAX_CONNECTIONS,
    DEFAULT_DB_JOBS_MAX_CONNECTIONS, DEFAULT_GRACEFUL_SHUTDOWN_SECS, DEFAULT_HOST,
    DEFAULT_JOB_PUBLISH_PAGE_CONCURRENCY, DEFAULT_JOB_PUBLISH_POST_CONCURRENCY,
    DEFAULT_JOB_RENDER_PAGE_CONCURRENCY, DEFAULT_JOB_RENDER_POST_CONCURRENCY,
    DEFAULT_JOB_RENDER_SUMMARY_CONCURRENCY, DEFAULT_MERMAID_CACHE_DIR, DEFAULT_MERMAID_CLI_PATH,
    DEFAULT_PUBLIC_PORT, DEFAULT_RATE_LIMIT_MAX_REQUESTS, DEFAULT_RATE_LIMIT_WINDOW_SECS,
    DEFAULT_SCHEDULER_CADENCE_SECS, DEFAULT_UPLOAD_DIR, DEFAULT_UPLOAD_REQUEST_LIMIT_BYTES,
    LOCAL_CONFIG_BASENAME,
};
use super::types::{
    ApiRateLimitSettings, CacheSettings, DatabaseSettings, JobsSettings, LoadError, LogFormat,
    LoggingSettings, RateLimitSettings, RenderSettings, SchedulerSettings, ServerSettings,
    Settings, UploadSettings,
};

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

/// Resolve configuration using the supplied CLI arguments, returning both for downstream use.
pub fn load_with_cli() -> Result<(CliArgs, Settings), LoadError> {
    let args = CliArgs::parse();
    let settings = load(&args)?;
    Ok((args, settings))
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
pub(super) struct RawSettings {
    pub(super) server: RawServerSettings,
    pub(super) logging: RawLoggingSettings,
    pub(super) database: RawDatabaseSettings,
    pub(super) jobs: RawJobsSettings,
    pub(super) render: RawRenderSettings,
    pub(super) uploads: RawUploadSettings,
    pub(super) rate_limit: RawRateLimitSettings,
    pub(super) api_rate_limit: RawApiRateLimitSettings,
    pub(super) scheduler: RawSchedulerSettings,
    pub(super) cache: RawCacheSettings,
}

impl Settings {
    pub(super) fn from_raw(raw: RawSettings) -> Result<Self, LoadError> {
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
            cache,
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
        let cache = build_cache_settings(cache)?;

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
            cache,
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

fn build_cache_settings(cache: RawCacheSettings) -> Result<CacheSettings, LoadError> {
    Ok(CacheSettings {
        enable_l0_cache: cache.enable_l0_cache.unwrap_or(true),
        enable_l1_cache: cache.enable_l1_cache.unwrap_or(true),
        l0_post_limit: cache.l0_post_limit.unwrap_or(DEFAULT_CACHE_L0_POST_LIMIT),
        l0_page_limit: cache.l0_page_limit.unwrap_or(DEFAULT_CACHE_L0_PAGE_LIMIT),
        l0_api_key_limit: cache
            .l0_api_key_limit
            .unwrap_or(DEFAULT_CACHE_L0_API_KEY_LIMIT),
        l0_post_list_limit: cache
            .l0_post_list_limit
            .unwrap_or(DEFAULT_CACHE_L0_POST_LIST_LIMIT),
        l1_response_limit: cache
            .l1_response_limit
            .unwrap_or(DEFAULT_CACHE_L1_RESPONSE_LIMIT),
        l1_response_body_limit_bytes: cache
            .l1_response_body_limit_bytes
            .unwrap_or(DEFAULT_CACHE_L1_RESPONSE_BODY_LIMIT_BYTES),
        auto_consume_interval_ms: cache
            .auto_consume_interval_ms
            .unwrap_or(DEFAULT_CACHE_AUTO_CONSUME_INTERVAL_MS),
        consume_batch_limit: cache
            .consume_batch_limit
            .unwrap_or(DEFAULT_CACHE_CONSUME_BATCH_LIMIT),
        max_event_queue_len: cache
            .max_event_queue_len
            .unwrap_or(DEFAULT_CACHE_MAX_EVENT_QUEUE_LEN),
    })
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

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
pub(super) struct RawServerSettings {
    pub(super) host: Option<String>,
    pub(super) admin_host: Option<String>,
    pub(super) public_port: Option<u16>,
    pub(super) admin_port: Option<u16>,
    pub(super) graceful_shutdown_seconds: Option<u64>,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
pub(super) struct RawLoggingSettings {
    pub(super) level: Option<String>,
    pub(super) json: Option<bool>,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
pub(super) struct RawDatabaseSettings {
    pub(super) url: Option<String>,
    pub(super) http_max_connections: Option<u32>,
    pub(super) jobs_max_connections: Option<u32>,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
pub(super) struct RawUploadSettings {
    pub(super) directory: Option<PathBuf>,
    pub(super) max_request_bytes: Option<u64>,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
pub(super) struct RawJobsSettings {
    pub(super) render_post_concurrency: Option<u32>,
    pub(super) render_summary_concurrency: Option<u32>,
    pub(super) render_page_concurrency: Option<u32>,
    pub(super) publish_post_concurrency: Option<u32>,
    pub(super) publish_page_concurrency: Option<u32>,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
pub(super) struct RawRenderSettings {
    pub(super) mermaid_cli_path: Option<PathBuf>,
    pub(super) mermaid_cache_dir: Option<PathBuf>,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
pub(super) struct RawRateLimitSettings {
    pub(super) window_seconds: Option<u64>,
    pub(super) max_requests: Option<u64>,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
pub(super) struct RawApiRateLimitSettings {
    pub(super) window_seconds: Option<u64>,
    pub(super) max_requests: Option<u64>,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
pub(super) struct RawSchedulerSettings {
    pub(super) cadence_seconds: Option<u64>,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
pub(super) struct RawCacheSettings {
    pub(super) enable_l0_cache: Option<bool>,
    pub(super) enable_l1_cache: Option<bool>,
    pub(super) l0_post_limit: Option<usize>,
    pub(super) l0_page_limit: Option<usize>,
    pub(super) l0_api_key_limit: Option<usize>,
    pub(super) l0_post_list_limit: Option<usize>,
    pub(super) l1_response_limit: Option<usize>,
    pub(super) l1_response_body_limit_bytes: Option<usize>,
    pub(super) auto_consume_interval_ms: Option<u64>,
    pub(super) consume_batch_limit: Option<usize>,
    pub(super) max_event_queue_len: Option<usize>,
}
