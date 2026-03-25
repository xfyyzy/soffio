use clap::{Parser, Subcommand};

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
