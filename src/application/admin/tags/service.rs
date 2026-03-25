use std::sync::Arc;

use crate::application::admin::audit::AdminAuditService;
use crate::application::repos::{TagsRepo, TagsWriteRepo};
use crate::cache::CacheTrigger;

#[derive(Clone)]
pub struct AdminTagService {
    pub(crate) reader: Arc<dyn TagsRepo>,
    pub(crate) writer: Arc<dyn TagsWriteRepo>,
    pub(crate) audit: AdminAuditService,
    pub(crate) cache_trigger: Option<Arc<CacheTrigger>>,
}

impl AdminTagService {
    pub fn new(
        reader: Arc<dyn TagsRepo>,
        writer: Arc<dyn TagsWriteRepo>,
        audit: AdminAuditService,
    ) -> Self {
        Self {
            reader,
            writer,
            audit,
            cache_trigger: None,
        }
    }

    /// Set the cache trigger for this service.
    pub fn with_cache_trigger(mut self, trigger: Arc<CacheTrigger>) -> Self {
        self.cache_trigger = Some(trigger);
        self
    }

    /// Set the cache trigger for this service (optional).
    pub fn with_cache_trigger_opt(mut self, trigger: Option<Arc<CacheTrigger>>) -> Self {
        self.cache_trigger = trigger;
        self
    }
}
