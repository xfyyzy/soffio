use std::path::PathBuf;

use clap::{Parser, Subcommand};
use uuid::Uuid;

use super::super::PageStatusArg;

#[derive(Parser, Debug)]
pub struct PagesArgs {
    #[command(subcommand)]
    pub action: PagesCmd,
}

#[derive(Subcommand, Debug)]
pub enum PagesCmd {
    /// List pages
    List {
        #[arg(long)]
        status: Option<PageStatusArg>,
        #[arg(long)]
        search: Option<String>,
        #[arg(long)]
        month: Option<String>,
        #[arg(long, default_value_t = 20)]
        limit: u32,
        #[arg(long)]
        cursor: Option<String>,
    },
    /// Get a page by id or slug
    Get {
        #[arg(long, required_unless_present = "slug", conflicts_with = "slug")]
        id: Option<Uuid>,
        #[arg(long, required_unless_present = "id", conflicts_with = "id")]
        slug: Option<String>,
    },
    /// Create a page
    Create {
        #[arg(long)]
        slug: Option<String>,
        #[arg(long)]
        title: String,
        #[arg(long)]
        body: Option<String>,
        #[arg(long)]
        body_file: Option<PathBuf>,
        #[arg(long, default_value_t = PageStatusArg::Draft)]
        status: PageStatusArg,
        #[arg(long)]
        scheduled_at: Option<String>,
        #[arg(long)]
        published_at: Option<String>,
        #[arg(long)]
        archived_at: Option<String>,
    },
    /// Update a page
    Update {
        #[arg(long)]
        id: Uuid,
        #[arg(long)]
        slug: String,
        #[arg(long)]
        title: String,
        #[arg(long)]
        body: Option<String>,
        #[arg(long)]
        body_file: Option<PathBuf>,
    },
    /// Patch title only
    PatchTitle {
        #[arg(long)]
        id: Uuid,
        #[arg(long)]
        title: String,
    },
    /// Patch body
    PatchBody {
        #[arg(long)]
        id: Uuid,
        #[arg(long)]
        body: Option<String>,
        #[arg(long)]
        body_file: Option<PathBuf>,
    },
    /// Update status and times
    Status {
        #[arg(long)]
        id: Uuid,
        #[arg(long)]
        status: PageStatusArg,
        #[arg(long)]
        scheduled_at: Option<String>,
        #[arg(long)]
        published_at: Option<String>,
        #[arg(long)]
        archived_at: Option<String>,
    },
    /// Delete a page
    Delete { id: Uuid },
}
