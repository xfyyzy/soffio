use std::sync::Arc;

use soffio::{
    application::error::AppError,
    config,
    infra::{db::PostgresRepositories, error::InfraError},
};

pub(super) async fn init_repositories(
    settings: &config::Settings,
) -> Result<(Arc<PostgresRepositories>, Arc<PostgresRepositories>), AppError> {
    let database_url = settings
        .database
        .url
        .as_ref()
        .ok_or_else(|| InfraError::configuration("database url is not configured"))
        .map_err(AppError::from)?;

    let http_pool =
        PostgresRepositories::connect(database_url, settings.database.http_max_connections.get())
            .await
            .map_err(|err| AppError::from(InfraError::database(err.to_string())))?;

    PostgresRepositories::run_migrations(&http_pool)
        .await
        .map_err(|err| AppError::from(InfraError::database(err.to_string())))?;

    let jobs_pool =
        PostgresRepositories::connect(database_url, settings.database.jobs_max_connections.get())
            .await
            .map_err(|err| AppError::from(InfraError::database(err.to_string())))?;

    Ok((
        Arc::new(PostgresRepositories::new(http_pool)),
        Arc::new(PostgresRepositories::new(jobs_pool)),
    ))
}
