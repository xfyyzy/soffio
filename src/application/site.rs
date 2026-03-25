//! Import/export of site content and configuration.

use std::{fs, path::Path};

use crate::{
    application::error::AppError,
    infra::{db::PostgresRepositories, error::InfraError},
};

#[path = "site/export.rs"]
mod export;
#[path = "site/import.rs"]
mod import;
#[path = "site/models.rs"]
mod models;

pub(super) const SETTINGS_ROW_ID: i16 = 1;

/// Export the current site data to the provided path as a TOML archive.
pub async fn export_site(repositories: &PostgresRepositories, path: &Path) -> Result<(), AppError> {
    let archive = export::gather_archive(repositories.pool()).await?;
    let encoded = toml::to_string_pretty(&archive)
        .map_err(|err| AppError::unexpected(format!("failed to encode archive: {err}")))?;
    fs::write(path, encoded).map_err(|err| AppError::from(InfraError::Io(err)))?;
    Ok(())
}

/// Import site data from the provided TOML archive path.
pub async fn import_site(repositories: &PostgresRepositories, path: &Path) -> Result<(), AppError> {
    let data = fs::read_to_string(path).map_err(|err| AppError::from(InfraError::Io(err)))?;
    let mut archive: models::SiteArchive = toml::from_str(&data)
        .map_err(|err| AppError::validation(format!("invalid archive: {err}")))?;
    archive.normalize();
    import::import_archive(repositories, archive).await
}

pub(super) fn map_sqlx_error(err: sqlx::Error) -> AppError {
    AppError::from(InfraError::database(err.to_string()))
}
