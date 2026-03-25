use clap::{Parser, Subcommand};
use uuid::Uuid;

use super::super::NavDestArg;

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
    /// Get a navigation item by id
    Get {
        #[arg(long)]
        id: Uuid,
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
