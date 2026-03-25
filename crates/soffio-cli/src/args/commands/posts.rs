use std::path::PathBuf;

use clap::{Parser, Subcommand};
use uuid::Uuid;

use super::super::PostStatusArg;

#[derive(Parser, Debug)]
pub struct PostsArgs {
    #[command(subcommand)]
    pub action: PostsCmd,
}

#[derive(Subcommand, Debug)]
pub enum PostsCmd {
    /// List posts with optional filters
    List {
        #[arg(long)]
        status: Option<PostStatusArg>,
        #[arg(long)]
        tag: Option<String>,
        #[arg(long)]
        search: Option<String>,
        #[arg(long)]
        month: Option<String>,
        #[arg(long, default_value_t = 20)]
        limit: u32,
        #[arg(long)]
        cursor: Option<String>,
    },
    /// Get a post by id or slug
    Get {
        #[arg(long, required_unless_present = "slug", conflicts_with = "slug")]
        id: Option<Uuid>,
        #[arg(long, required_unless_present = "id", conflicts_with = "id")]
        slug: Option<String>,
    },
    /// Create a post
    Create {
        #[arg(long)]
        title: String,
        #[arg(long)]
        excerpt: String,
        #[arg(long)]
        body: Option<String>,
        #[arg(long)]
        body_file: Option<PathBuf>,
        #[arg(long)]
        summary: Option<String>,
        #[arg(long)]
        summary_file: Option<PathBuf>,
        #[arg(long, default_value_t = PostStatusArg::Draft)]
        status: PostStatusArg,
        #[arg(long, default_value_t = false)]
        pinned: bool,
        #[arg(long)]
        scheduled_at: Option<String>,
        #[arg(long)]
        published_at: Option<String>,
        #[arg(long)]
        archived_at: Option<String>,
    },
    /// Update all mutable fields of a post
    Update {
        #[arg(long)]
        id: Uuid,
        #[arg(long)]
        slug: String,
        #[arg(long)]
        title: String,
        #[arg(long)]
        excerpt: String,
        #[arg(long)]
        body: Option<String>,
        #[arg(long)]
        body_file: Option<PathBuf>,
        #[arg(long)]
        summary: Option<String>,
        #[arg(long)]
        summary_file: Option<PathBuf>,
        #[arg(long, default_value_t = false)]
        pinned: bool,
    },
    /// Patch title only
    PatchTitle {
        #[arg(long)]
        id: Uuid,
        #[arg(long)]
        title: String,
    },
    /// Patch excerpt
    PatchExcerpt {
        #[arg(long)]
        id: Uuid,
        #[arg(long)]
        excerpt: String,
    },
    /// Patch body (supports file input)
    PatchBody {
        #[arg(long)]
        id: Uuid,
        #[arg(long)]
        body: Option<String>,
        #[arg(long)]
        body_file: Option<PathBuf>,
    },
    /// Patch summary (supports file input)
    PatchSummary {
        #[arg(long)]
        id: Uuid,
        #[arg(long)]
        summary: Option<String>,
        #[arg(long)]
        summary_file: Option<PathBuf>,
    },
    /// Update status and schedule times
    Status {
        #[arg(long)]
        id: Uuid,
        #[arg(long)]
        status: PostStatusArg,
        #[arg(long)]
        scheduled_at: Option<String>,
        #[arg(long)]
        published_at: Option<String>,
        #[arg(long)]
        archived_at: Option<String>,
    },
    /// Replace tag list
    Tags {
        #[arg(long)]
        id: Uuid,
        #[arg(long)]
        tag_ids: String,
    },
    /// Pin or unpin
    Pin {
        #[arg(long)]
        id: Uuid,
        #[arg(long, default_value_t = true)]
        pinned: bool,
    },
    /// Delete a post
    Delete { id: Uuid },
}
