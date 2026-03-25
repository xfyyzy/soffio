use std::path::PathBuf;

use clap::{Parser, Subcommand};
use uuid::Uuid;

#[derive(Parser, Debug)]
pub struct UploadsArgs {
    #[command(subcommand)]
    pub action: UploadsCmd,
}

#[derive(Subcommand, Debug)]
pub enum UploadsCmd {
    /// List uploads
    List {
        #[arg(long)]
        content_type: Option<String>,
        #[arg(long)]
        search: Option<String>,
        #[arg(long)]
        month: Option<String>,
        #[arg(long, default_value_t = 20)]
        limit: u32,
        #[arg(long)]
        cursor: Option<String>,
    },
    /// Get an upload by id
    Get {
        #[arg(long)]
        id: Uuid,
    },
    /// Upload a file
    Upload { file: PathBuf },
    /// Delete an upload
    Delete { id: Uuid },
}
