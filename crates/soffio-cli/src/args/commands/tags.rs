use std::path::PathBuf;

use clap::{Parser, Subcommand};
use uuid::Uuid;

#[derive(Parser, Debug)]
pub struct TagsArgs {
    #[command(subcommand)]
    pub action: TagsCmd,
}

#[derive(Subcommand, Debug)]
pub enum TagsCmd {
    /// List tags
    List {
        #[arg(long)]
        pinned: Option<bool>,
        #[arg(long)]
        search: Option<String>,
        #[arg(long)]
        month: Option<String>,
        #[arg(long, default_value_t = 20)]
        limit: u32,
        #[arg(long)]
        cursor: Option<String>,
    },
    /// Get a tag by id or slug
    Get {
        #[arg(long, required_unless_present = "slug", conflicts_with = "slug")]
        id: Option<Uuid>,
        #[arg(long, required_unless_present = "id", conflicts_with = "id")]
        slug: Option<String>,
    },
    /// Create a tag
    Create {
        #[arg(long)]
        name: String,
        #[arg(long)]
        description: Option<String>,
        #[arg(long)]
        description_file: Option<PathBuf>,
        #[arg(long, default_value_t = false)]
        pinned: bool,
    },
    /// Update all fields
    Update {
        #[arg(long)]
        id: Uuid,
        #[arg(long)]
        name: String,
        #[arg(long)]
        description: Option<String>,
        #[arg(long)]
        description_file: Option<PathBuf>,
        #[arg(long, default_value_t = false)]
        pinned: bool,
    },
    /// Pin or unpin
    PatchPin {
        #[arg(long)]
        id: Uuid,
        #[arg(long, default_value_t = true)]
        pinned: bool,
    },
    /// Update name only
    PatchName {
        #[arg(long)]
        id: Uuid,
        #[arg(long)]
        name: String,
    },
    /// Update description only (supports file input)
    PatchDescription {
        #[arg(long)]
        id: Uuid,
        #[arg(long)]
        description: Option<String>,
        #[arg(long)]
        description_file: Option<PathBuf>,
    },
    /// Delete a tag
    Delete { id: Uuid },
}
