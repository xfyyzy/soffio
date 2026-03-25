use std::sync::Arc;

use apalis::{
    layers::WorkerBuilderExt,
    prelude::{Monitor, WorkerBuilder, WorkerFactoryFn},
};
use apalis_cron::CronStream;
use apalis_sql::{Config as ApalisSqlConfig, postgres::PostgresStorage};
use soffio::{
    application::{
        api_keys::ApiKeyService,
        jobs::{
            ExpireApiKeysContext, JobWorkerContext, expire_api_keys_schedule,
            process_expire_api_keys_job, process_publish_page_job, process_publish_post_job,
        },
        render::{process_render_page_job, process_render_post_job},
    },
    config,
    domain::types::JobType,
    infra::db::PostgresRepositories,
};
use tracing::error;

pub(super) fn spawn_job_monitor(
    repositories: Arc<PostgresRepositories>,
    context: JobWorkerContext,
    api_keys: Arc<ApiKeyService>,
    jobs: &config::JobsSettings,
) -> tokio::task::JoinHandle<()> {
    let render_storage = PostgresStorage::new_with_config(
        repositories.pool().clone(),
        ApalisSqlConfig::new(JobType::RenderPost.as_str()),
    );
    let render_page_storage = PostgresStorage::new_with_config(
        repositories.pool().clone(),
        ApalisSqlConfig::new(JobType::RenderPage.as_str()),
    );
    let publish_post_storage = PostgresStorage::new_with_config(
        repositories.pool().clone(),
        ApalisSqlConfig::new(JobType::PublishPost.as_str()),
    );
    let publish_page_storage = PostgresStorage::new_with_config(
        repositories.pool().clone(),
        ApalisSqlConfig::new(JobType::PublishPage.as_str()),
    );

    let render_post_concurrency = jobs.render_post_concurrency.get() as usize;
    let render_page_concurrency = jobs.render_page_concurrency.get() as usize;
    let publish_post_concurrency = jobs.publish_post_concurrency.get() as usize;
    let publish_page_concurrency = jobs.publish_page_concurrency.get() as usize;

    let render_post_worker = WorkerBuilder::new("render-post-worker")
        .concurrency(render_post_concurrency)
        .data(context.clone())
        .backend(render_storage)
        .build_fn(process_render_post_job);
    let render_page_worker = WorkerBuilder::new("render-page-worker")
        .concurrency(render_page_concurrency)
        .data(context.clone())
        .backend(render_page_storage)
        .build_fn(process_render_page_job);
    let publish_post_worker = WorkerBuilder::new("publish-post-worker")
        .concurrency(publish_post_concurrency)
        .data(context.clone())
        .backend(publish_post_storage)
        .build_fn(process_publish_post_job);
    let publish_page_worker = WorkerBuilder::new("publish-page-worker")
        .concurrency(publish_page_concurrency)
        .data(context.clone())
        .backend(publish_page_storage)
        .build_fn(process_publish_page_job);

    let expire_api_keys_ctx = ExpireApiKeysContext { api_keys };
    let expire_api_keys_worker = WorkerBuilder::new("expire-api-keys-worker")
        .data(expire_api_keys_ctx)
        .backend(CronStream::new(expire_api_keys_schedule()))
        .build_fn(process_expire_api_keys_job);

    let monitor = Monitor::new()
        .register(render_post_worker)
        .register(render_page_worker)
        .register(publish_post_worker)
        .register(publish_page_worker)
        .register(expire_api_keys_worker);

    tokio::spawn(async move {
        if let Err(err) = monitor.run().await {
            error!(error = %err, "job monitor stopped");
        }
    })
}
