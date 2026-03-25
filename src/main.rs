use std::process;

use soffio::{
    application::error::AppError,
    application::render::{RenderPipelineConfig, configure_render_service},
    config,
    infra::telemetry,
};
use tracing::{Dispatch, Level, dispatcher, error};
use tracing_subscriber::fmt as tracing_fmt;

#[path = "main/import_export.rs"]
mod import_export;
mod migrations_tool;
#[path = "main/renderall.rs"]
mod renderall;
#[path = "main/serve.rs"]
mod serve;

#[tokio::main]
async fn main() {
    if let Err(error) = run().await {
        report_application_error(&error);
        process::exit(1);
    }
}

fn report_application_error(error: &AppError) {
    if dispatcher::has_been_set() {
        error!(error = %error, "application error");
        return;
    }

    let subscriber = tracing_fmt().with_max_level(Level::ERROR).finish();
    let dispatch = Dispatch::new(subscriber);
    dispatcher::with_default(&dispatch, || {
        error!(error = %error, "application error");
    });
}

async fn run() -> Result<(), AppError> {
    let (cli_args, settings) = config::load_with_cli()
        .map_err(|err| AppError::unexpected(format!("failed to load configuration: {err}")))?;

    let command = cli_args
        .command
        .unwrap_or(config::Command::Serve(Box::<config::ServeArgs>::default()));

    telemetry::init(&settings.logging).map_err(AppError::from)?;
    configure_render_service(RenderPipelineConfig::from(&settings.render))
        .map_err(|err| AppError::unexpected(err.to_string()))?;

    match command {
        config::Command::Serve(_) => serve::run_serve(settings).await,
        config::Command::RenderAll(args) => renderall::run_renderall(settings, args).await,
        config::Command::ExportSite(args) => import_export::run_export_site(settings, args).await,
        config::Command::ImportSite(args) => import_export::run_import_site(settings, args).await,
        config::Command::Migrations(args) => import_export::run_migrations(settings, args).await,
    }
}

#[cfg(test)]
mod tests {}
