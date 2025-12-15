use std::sync::Arc;

use crate::application::admin::{
    audit::AdminAuditService,
    snapshot_types::{PostSnapshotPayload, PostSnapshotSource},
};
use crate::application::repos::{
    JobsRepo, PostsRepo, PostsWriteRepo, RestorePostSnapshotParams, SectionsRepo, TagsRepo,
};
use crate::cache::CacheTrigger;
use crate::domain::entities::PostRecord;

#[derive(Clone)]
pub struct AdminPostService {
    pub(crate) reader: Arc<dyn PostsRepo>,
    pub(crate) writer: Arc<dyn PostsWriteRepo>,
    pub(crate) sections: Arc<dyn SectionsRepo>,
    pub(crate) jobs: Arc<dyn JobsRepo>,
    pub(crate) tags: Arc<dyn TagsRepo>,
    pub(crate) audit: AdminAuditService,
    pub(crate) cache_trigger: Option<Arc<CacheTrigger>>,
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

    pub async fn snapshot_source(
        &self,
        id: uuid::Uuid,
    ) -> Result<PostSnapshotSource, crate::application::admin::posts::types::AdminPostError> {
        use crate::application::admin::posts::types::AdminPostError;

        let post = self
            .reader
            .find_by_id(id)
            .await?
            .ok_or(AdminPostError::Repo(
                crate::application::repos::RepoError::NotFound,
            ))?;

        let tags = self.tags.list_for_post(id).await?;
        let sections = self.sections.list_sections(id).await?;

        let tag_ids: Vec<uuid::Uuid> = tags.into_iter().map(|t| t.id).collect();

        Ok(PostSnapshotSource {
            post,
            tags: tag_ids,
            sections,
        })
    }

    pub async fn restore_from_snapshot(
        &self,
        payload: PostSnapshotPayload,
        post_id: uuid::Uuid,
    ) -> Result<PostRecord, crate::application::admin::posts::types::AdminPostError> {
        let params = RestorePostSnapshotParams {
            id: post_id,
            slug: payload.slug,
            title: payload.title,
            excerpt: payload.excerpt,
            body_markdown: payload.body_markdown,
            summary_markdown: payload.summary_markdown,
            summary_html: payload.summary_html,
            status: payload.status,
            pinned: payload.pinned,
            scheduled_at: payload.scheduled_at,
            published_at: payload.published_at,
            archived_at: payload.archived_at,
            tag_ids: payload.tags,
            sections: payload.sections,
        };

        let post = self.writer.restore_post_snapshot(params).await?;
        Ok(post)
    }
}
