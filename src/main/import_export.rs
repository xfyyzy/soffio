use soffio::{application::error::AppError, application::site, config};
use tracing::info;

use crate::migrations_tool;
use crate::serve::init_repositories;

pub(super) async fn run_export_site(
    settings: config::Settings,
    args: config::ExportArgs,
) -> Result<(), AppError> {
    let (http_repositories, _) = init_repositories(&settings).await?;
    let path = args.file;

    info!(
        target = "soffio::export",
        path = %path.display(),
        "Starting export"
    );

    site::export_site(&http_repositories, &path).await?;
    info!(target = "soffio::export", "Export completed");
    Ok(())
}

pub(super) async fn run_import_site(
    settings: config::Settings,
    args: config::ImportArgs,
) -> Result<(), AppError> {
    let (http_repositories, _) = init_repositories(&settings).await?;
    let path = args.file;

    info!(
        target = "soffio::import",
        path = %path.display(),
        "Starting import"
    );

    site::import_site(&http_repositories, &path).await?;
    info!(
        target = "soffio::import",
        "Import completed. Re-run renderall to regenerate derived content."
    );
    Ok(())
}

pub(super) async fn run_migrations(
    settings: config::Settings,
    args: config::MigrationsArgs,
) -> Result<(), AppError> {
    match args.command {
        config::MigrationsCommand::Reconcile(cmd) => {
            migrations_tool::reconcile_archive(&settings.database, &cmd).await?
        }
    }
    Ok(())
}
