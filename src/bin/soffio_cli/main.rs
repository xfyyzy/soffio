//! soffio-cli: headless API command-line client
//! Modularized for maintainability; reuses infra http models for request/response shapes.
#![deny(clippy::all, clippy::pedantic)]

mod args;
mod client;
mod handlers;
mod io;
mod print;

use clap::Parser;

use args::{Cli, Commands};
use client::{CliError, build_ctx_from_cli};
use handlers::{api_keys, audit, jobs, navigation, pages, posts, settings, tags, uploads};

#[tokio::main]
async fn main() -> Result<(), CliError> {
    let cli = Cli::parse();
    let ctx = build_ctx_from_cli(&cli)?;

    match cli.command {
        Commands::ApiKeys(cmd) => api_keys::handle(&ctx, cmd).await?,
        Commands::Posts(cmd) => posts::handle(&ctx, cmd.action).await?,
        Commands::Pages(cmd) => pages::handle(&ctx, cmd.action).await?,
        Commands::Tags(cmd) => tags::handle(&ctx, cmd.action).await?,
        Commands::Navigation(cmd) => navigation::handle(&ctx, cmd.action).await?,
        Commands::Uploads(cmd) => uploads::handle(&ctx, cmd.action).await?,
        Commands::Settings(cmd) => settings::handle(&ctx, cmd.action).await?,
        Commands::Jobs(cmd) => jobs::handle(&ctx, cmd.action).await?,
        Commands::Audit(cmd) => audit::handle(&ctx, cmd.action).await?,
    }

    Ok(())
}
