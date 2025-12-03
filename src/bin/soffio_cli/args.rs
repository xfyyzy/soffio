//! Command-line surface for `soffio-cli`.
//! Kept in a shared file so doc generators and tests can reuse the same
//! definitions as the binary itself.

#![deny(clippy::all, clippy::pedantic)]

use std::fmt;
use std::path::PathBuf;

use clap::{Parser, Subcommand, ValueEnum};
use uuid::Uuid;

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
}

#[derive(Parser, Debug)]
pub struct ApiKeysCmd {
    #[command(subcommand)]
    pub action: ApiKeysAction,
}

#[derive(Subcommand, Debug)]
pub enum ApiKeysAction {
    /// Show current API key metadata/scopes
    Me,
}

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
    /// Get a post by slug
    Get { slug: String },
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
    /// Patch only title and slug
    PatchTitleSlug {
        #[arg(long)]
        id: Uuid,
        #[arg(long)]
        title: Option<String>,
        #[arg(long)]
        slug: Option<String>,
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
    /// Get by slug
    Get { slug: String },
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
    /// Patch title/slug only
    PatchTitleSlug {
        #[arg(long)]
        id: Uuid,
        #[arg(long)]
        title: Option<String>,
        #[arg(long)]
        slug: Option<String>,
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

#[derive(Parser, Debug)]
pub struct NavArgs {
    #[command(subcommand)]
    pub action: NavCmd,
}

#[derive(Subcommand, Debug)]
pub enum NavCmd {
    /// List navigation items
    List {
        #[arg(long)]
        visible: Option<bool>,
        #[arg(long)]
        search: Option<String>,
        #[arg(long, default_value_t = 20)]
        limit: u32,
        #[arg(long)]
        cursor: Option<String>,
    },
    /// Create a navigation entry
    Create {
        #[arg(long)]
        label: String,
        #[arg(long, value_enum)]
        destination_type: NavDestArg,
        #[arg(long)]
        destination_page_id: Option<Uuid>,
        #[arg(long)]
        destination_url: Option<String>,
        #[arg(long)]
        sort_order: i32,
        #[arg(long, default_value_t = false)]
        visible: bool,
        #[arg(long, default_value_t = false)]
        open_in_new_tab: bool,
    },
    /// Update all navigation fields
    Update {
        #[arg(long)]
        id: Uuid,
        #[arg(long)]
        label: String,
        #[arg(long, value_enum)]
        destination_type: NavDestArg,
        #[arg(long)]
        destination_page_id: Option<Uuid>,
        #[arg(long)]
        destination_url: Option<String>,
        #[arg(long)]
        sort_order: i32,
        #[arg(long, default_value_t = false)]
        visible: bool,
        #[arg(long, default_value_t = false)]
        open_in_new_tab: bool,
    },
    /// Patch label only
    PatchLabel { id: Uuid, label: String },
    /// Patch destination
    PatchDestination {
        id: Uuid,
        #[arg(long, value_enum)]
        destination_type: NavDestArg,
        #[arg(long)]
        destination_page_id: Option<Uuid>,
        #[arg(long)]
        destination_url: Option<String>,
    },
    /// Patch sort order
    PatchSort { id: Uuid, sort_order: i32 },
    /// Patch visibility
    PatchVisibility { id: Uuid, visible: bool },
    /// Patch open-in-new-tab flag
    PatchOpen { id: Uuid, open_in_new_tab: bool },
    /// Delete a navigation entry
    Delete { id: Uuid },
}

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
    /// Upload a file
    Upload { file: PathBuf },
    /// Delete an upload
    Delete { id: Uuid },
}

#[derive(Parser, Debug)]
pub struct SettingsArgs {
    #[command(subcommand)]
    pub action: SettingsCmd,
}

#[derive(Subcommand, Debug)]
pub enum SettingsCmd {
    /// Show settings
    Get,
    /// Patch settings (only provided fields)
    Patch(Box<SettingsPatchArgs>),
}

#[derive(Parser, Debug)]
pub struct SettingsPatchArgs {
    #[arg(long)]
    pub brand_title: Option<String>,
    #[arg(long)]
    pub brand_href: Option<String>,
    #[arg(long)]
    pub footer_copy: Option<String>,
    #[arg(long)]
    pub homepage_size: Option<i32>,
    #[arg(long)]
    pub admin_page_size: Option<i32>,
    #[arg(long)]
    pub show_tag_aggregations: Option<bool>,
    #[arg(long)]
    pub show_month_aggregations: Option<bool>,
    #[arg(long)]
    pub tag_filter_limit: Option<i32>,
    #[arg(long)]
    pub month_filter_limit: Option<i32>,
    #[arg(long)]
    pub timezone: Option<String>,
    #[arg(long)]
    pub meta_title: Option<String>,
    #[arg(long)]
    pub meta_description: Option<String>,
    #[arg(long)]
    pub og_title: Option<String>,
    #[arg(long)]
    pub og_description: Option<String>,
    #[arg(long)]
    pub public_site_url: Option<String>,
    #[arg(long)]
    pub global_toc_enabled: Option<bool>,
    #[arg(long)]
    pub favicon_svg: Option<String>,
    #[arg(long)]
    pub favicon_svg_file: Option<PathBuf>,
}

#[derive(Parser, Debug)]
pub struct JobsArgs {
    #[command(subcommand)]
    pub action: JobsCmd,
}

#[derive(Subcommand, Debug)]
pub enum JobsCmd {
    /// List background jobs
    List {
        #[arg(long)]
        state: Option<String>,
        #[arg(long)]
        job_type: Option<String>,
        #[arg(long)]
        search: Option<String>,
        #[arg(long, default_value_t = 20)]
        limit: u32,
        #[arg(long)]
        cursor: Option<String>,
    },
}

#[derive(Parser, Debug)]
pub struct AuditArgs {
    #[command(subcommand)]
    pub action: AuditCmd,
}

#[derive(Subcommand, Debug)]
pub enum AuditCmd {
    /// List audit logs
    List {
        #[arg(long)]
        actor: Option<String>,
        #[arg(long)]
        action: Option<String>,
        #[arg(long)]
        entity_type: Option<String>,
        #[arg(long)]
        search: Option<String>,
        #[arg(long, default_value_t = 20)]
        limit: u32,
        #[arg(long)]
        cursor: Option<String>,
    },
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum PostStatusArg {
    Draft,
    Published,
    Archived,
    Error,
}

impl PostStatusArg {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Draft => "draft",
            Self::Published => "published",
            Self::Archived => "archived",
            Self::Error => "error",
        }
    }
}

impl fmt::Display for PostStatusArg {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum PageStatusArg {
    Draft,
    Published,
    Archived,
    Error,
}

impl PageStatusArg {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Draft => "draft",
            Self::Published => "published",
            Self::Archived => "archived",
            Self::Error => "error",
        }
    }
}

impl fmt::Display for PageStatusArg {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum NavDestArg {
    Internal,
    External,
}
