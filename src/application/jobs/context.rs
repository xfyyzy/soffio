use std::{sync::Arc, time::Duration};

use apalis::prelude::Error as ApalisError;

use crate::{
    application::render::{InFlightRenders, RenderMailbox},
    application::{
        chrome::ChromeService, feed::FeedService, page::PageService, render::ComrakRenderService,
    },
    infra::{cache::ResponseCache, db::PostgresRepositories, uploads::UploadStorage},
};

pub const PUBLISH_JOB_WAIT_TIMEOUT: Duration = Duration::from_secs(30);
pub const CACHE_INVALIDATION_WAIT_TIMEOUT: Duration = Duration::from_secs(5);

/// Shared context passed to job workers so they can access infrastructure capabilities.
#[derive(Clone)]
pub struct JobWorkerContext {
    pub repositories: Arc<PostgresRepositories>,
    pub renderer: Arc<ComrakRenderService>,
    pub cache: Arc<ResponseCache>,
    pub feed: Arc<FeedService>,
    pub pages: Arc<PageService>,
    pub chrome: Arc<ChromeService>,
    pub upload_storage: Arc<UploadStorage>,
    pub render_mailbox: RenderMailbox,
    pub inflight_renders: InFlightRenders,
}

type BoxError = Box<dyn std::error::Error + Send + Sync + 'static>;

/// Convert any error into an [`ApalisError::Failed`].
pub fn job_failed<E>(err: E) -> ApalisError
where
    E: std::error::Error + Send + Sync + 'static,
{
    let boxed: BoxError = Box::new(err);
    ApalisError::Failed(Arc::new(boxed))
}
