//! Command-line surface for `soffio-cli`.
//! Kept in a shared file so doc generators and tests can reuse the same
//! definitions as the binary itself.

#![deny(clippy::all, clippy::pedantic)]

use std::path::PathBuf;

use clap::{Parser, Subcommand};

#[path = "args/commands.rs"]
mod commands;
#[path = "args/value_enums.rs"]
mod value_enums;

pub use commands::*;
pub use value_enums::*;

#[derive(Parser, Debug)]
#[command(name = "soffio-cli", version, about = "Soffio headless API CLI", long_about = None)]
pub struct Cli {
    /// API base URL, e.g. <https://example.com>
    #[arg(long, env = "SOFFIO_SITE_URL")]
    pub site: Option<String>,

    /// Path to file containing API key (takes precedence over env)
    #[arg(long, env = "SOFFIO_API_KEY_FILE")]
    pub key_file: Option<PathBuf>,

    /// API key from env (CLI flag intentionally disabled to avoid shell history leaks)
    #[arg(hide = true, env = "SOFFIO_API_KEY")]
    pub api_key_env: Option<String>,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// API key inspection
    ApiKeys(ApiKeysCmd),
    /// Post management (list/read/write/status/tags)
    Posts(PostsArgs),
    /// Page management
    Pages(PagesArgs),
    /// Tag management
    Tags(TagsArgs),
    /// Navigation menu management
    Navigation(NavArgs),
    /// Asset uploads
    Uploads(UploadsArgs),
    /// Site-wide settings
    Settings(SettingsArgs),
    /// Background jobs
    Jobs(JobsArgs),
    /// Audit log access
    Audit(AuditArgs),
    /// Snapshots management
    Snapshots(SnapshotsArgs),
}
