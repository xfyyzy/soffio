use std::sync::Arc;

use uuid::Uuid;

use crate::{
    application::admin::{
        audit::AdminAuditService,
        snapshot_types::{PageSnapshotPayload, PageSnapshotSource},
    },
    application::repos::{
        JobsRepo, PagesRepo, PagesWriteRepo, RepoError, RestorePageSnapshotParams, SettingsRepo,
    },
    cache::CacheTrigger,
    domain::entities::PageRecord,
};

use super::types::AdminPageError;

#[derive(Clone)]
pub struct AdminPageService {
    pub(crate) reader: Arc<dyn PagesRepo>,
    pub(crate) writer: Arc<dyn PagesWriteRepo>,
    pub(crate) jobs: Arc<dyn JobsRepo>,
    pub(crate) audit: AdminAuditService,
    pub(crate) settings: Arc<dyn SettingsRepo>,
    pub(crate) cache_trigger: Option<Arc<CacheTrigger>>,
}

impl AdminPageService {
    pub fn new(
        reader: Arc<dyn PagesRepo>,
        writer: Arc<dyn PagesWriteRepo>,
        jobs: Arc<dyn JobsRepo>,
        audit: AdminAuditService,
        settings: Arc<dyn SettingsRepo>,
    ) -> Self {
        Self {
            reader,
            writer,
            jobs,
            audit,
            settings,
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

    pub async fn snapshot_source(&self, id: Uuid) -> Result<PageSnapshotSource, AdminPageError> {
        let page = self
            .reader
            .find_by_id(id)
            .await?
            .ok_or(AdminPageError::Repo(RepoError::NotFound))?;

        Ok(PageSnapshotSource { page })
    }

    pub async fn restore_from_snapshot(
        &self,
        payload: PageSnapshotPayload,
        page_id: Uuid,
    ) -> Result<PageRecord, AdminPageError> {
        let previous_slug = self
            .reader
            .find_by_id(page_id)
            .await?
            .map(|record| record.slug);
        let params = RestorePageSnapshotParams {
            id: page_id,
            slug: payload.slug,
            title: payload.title,
            body_markdown: payload.body_markdown,
            rendered_html: payload.rendered_html,
            status: payload.status,
            scheduled_at: payload.scheduled_at,
            published_at: payload.published_at,
            archived_at: payload.archived_at,
        };

        let page = self.writer.restore_page_snapshot(params).await?;

        // Trigger cache invalidation
        if let Some(trigger) = &self.cache_trigger {
            let previous_slug = previous_slug.filter(|slug| slug != &page.slug);
            trigger
                .page_upserted_with_previous_slug(page.id, &page.slug, previous_slug.as_deref())
                .await;
        }

        Ok(page)
    }

    /// Trigger cache invalidation after background materialization completes.
    pub(crate) async fn notify_page_materialized(
        &self,
        page_id: Uuid,
        slug: &str,
    ) -> Result<(), AdminPageError> {
        let previous_slug = self
            .reader
            .find_by_id(page_id)
            .await?
            .map(|record| record.slug);
        if let Some(trigger) = &self.cache_trigger {
            let previous_slug = previous_slug.filter(|value| value != slug);
            trigger
                .page_upserted_with_previous_slug(page_id, slug, previous_slug.as_deref())
                .await;
        }
        Ok(())
    }
}
