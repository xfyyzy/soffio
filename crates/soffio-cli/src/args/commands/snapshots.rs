use clap::{Parser, Subcommand};
use uuid::Uuid;

#[derive(Parser, Debug)]
pub struct SnapshotsArgs {
    #[command(subcommand)]
    pub action: SnapshotsCmd,
}

#[derive(Subcommand, Debug)]
pub enum SnapshotsCmd {
    /// List snapshots
    List {
        #[arg(long)]
        entity_type: Option<String>,
        #[arg(long)]
        entity_id: Option<Uuid>,
        #[arg(long)]
        search: Option<String>,
        #[arg(long, default_value_t = 20)]
        limit: u32,
        #[arg(long)]
        cursor: Option<String>,
    },
    /// Get a snapshot
    Get { id: Uuid },
    /// Create a snapshot
    Create {
        #[arg(long)]
        entity_type: String,
        #[arg(long)]
        entity_id: Uuid,
        #[arg(long)]
        description: Option<String>,
    },
    /// Rollback to a snapshot
    Rollback { id: Uuid },
}
