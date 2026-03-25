//! Configuration layer: typed settings with layered precedence (file → env → CLI).

mod cli;
mod defaults;
mod loading;
mod overrides;
mod types;

pub use cli::{
    CliArgs, Command, DatabaseOverride, ExportArgs, ImportArgs, MigrationsArgs, MigrationsCommand,
    MigrationsReconcileArgs, RenderAllArgs, RenderAllOverrides, RenderOverrides, ServeArgs,
    ServeOverrides,
};
pub(crate) use defaults::{DEFAULT_MERMAID_CACHE_DIR, DEFAULT_MERMAID_CLI_PATH};
pub use loading::{load, load_with_cli};
pub use types::{
    ApiRateLimitSettings, CacheSettings, DatabaseSettings, JobsSettings, LoadError, LogFormat,
    LoggingSettings, RateLimitSettings, RenderSettings, SchedulerSettings, ServerSettings,
    Settings, UploadSettings,
};

#[cfg(test)]
mod tests;
