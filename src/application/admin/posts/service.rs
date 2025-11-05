use std::sync::Arc;

use crate::application::admin::audit::AdminAuditService;
use crate::application::repos::{JobsRepo, PostsRepo, PostsWriteRepo, SectionsRepo, TagsRepo};

#[derive(Clone)]
pub struct AdminPostService {
    pub(crate) reader: Arc<dyn PostsRepo>,
    pub(crate) writer: Arc<dyn PostsWriteRepo>,
    pub(crate) sections: Arc<dyn SectionsRepo>,
    pub(crate) jobs: Arc<dyn JobsRepo>,
    pub(crate) tags: Arc<dyn TagsRepo>,
    pub(crate) audit: AdminAuditService,
}

impl AdminPostService {
    pub fn new(
        reader: Arc<dyn PostsRepo>,
        writer: Arc<dyn PostsWriteRepo>,
        sections: Arc<dyn SectionsRepo>,
        jobs: Arc<dyn JobsRepo>,
        tags: Arc<dyn TagsRepo>,
        audit: AdminAuditService,
    ) -> Self {
        Self {
            reader,
            writer,
            sections,
            jobs,
            tags,
            audit,
        }
    }
}
