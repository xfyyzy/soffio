use clap::{Parser, Subcommand};

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
