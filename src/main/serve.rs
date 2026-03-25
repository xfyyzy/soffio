use std::sync::Arc;

use soffio::{
    application::{api_keys::ApiKeyService, error::AppError, jobs::JobWorkerContext},
    cache::CacheTrigger,
    config,
    infra::{
        db::PostgresRepositories,
        http::{AdminState, ApiState, HttpState},
    },
};

#[path = "serve/context.rs"]
mod context;
#[path = "serve/http_server.rs"]
mod http_server;
#[path = "serve/job_monitor.rs"]
mod job_monitor;
#[path = "serve/repositories.rs"]
mod repositories;

pub(super) struct ApplicationContext {
    pub(super) http_state: HttpState,
    pub(super) admin_state: AdminState,
    pub(super) api_state: ApiState,
    pub(super) job_context: JobWorkerContext,
    pub(super) api_keys: Arc<ApiKeyService>,
    pub(super) cache_trigger: Option<Arc<CacheTrigger>>,
}

use http_server::serve_http;
use job_monitor::spawn_job_monitor;

pub(super) async fn run_serve(settings: config::Settings) -> Result<(), AppError> {
    let (http_repositories, job_repositories) = init_repositories(&settings).await?;
    let app = build_application_context(
        http_repositories.clone(),
        job_repositories.clone(),
        &settings,
    )?;

    if let Some(trigger) = &app.cache_trigger {
        trigger.warmup_on_startup().await;
    }

    let cache_handle = if let Some(trigger) = app.cache_trigger.clone() {
        let interval_ms = trigger.config().auto_consume_interval_ms;
        Some(tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_millis(interval_ms));
            interval.tick().await;
            loop {
                interval.tick().await;
                trigger.consumer().consume_full().await;
            }
        }))
    } else {
        None
    };

    let monitor_handle = spawn_job_monitor(
        job_repositories,
        app.job_context.clone(),
        app.api_keys.clone(),
        &settings.jobs,
    );

    let result = serve_http(&settings, app.http_state, app.admin_state, app.api_state).await;

    monitor_handle.abort();
    let _ = monitor_handle.await;

    if let Some(h) = cache_handle {
        h.abort();
        let _ = h.await;
    }

    result
}

pub(super) async fn init_repositories(
    settings: &config::Settings,
) -> Result<(Arc<PostgresRepositories>, Arc<PostgresRepositories>), AppError> {
    repositories::init_repositories(settings).await
}

pub(super) fn build_application_context(
    http_repositories: Arc<PostgresRepositories>,
    job_repositories: Arc<PostgresRepositories>,
    settings: &config::Settings,
) -> Result<ApplicationContext, AppError> {
    context::build_application_context(http_repositories, job_repositories, settings)
}
