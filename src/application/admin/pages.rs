use std::sync::Arc;

use serde::Serialize;
use thiserror::Error;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::application::admin::audit::AdminAuditService;
use crate::application::admin::snapshot_types::{PageSnapshotPayload, PageSnapshotSource};
use crate::application::jobs::{
    PUBLISH_JOB_WAIT_TIMEOUT, enqueue_publish_page_job, wait_for_job_completion,
};
use crate::application::pagination::{CursorPage, PageCursor};
use crate::application::render::{
    RenderError, RenderRequest, RenderService, RenderTarget, enqueue_render_page_job,
    render_service,
};
use crate::application::repos::{
    CreatePageParams, JobsRepo, PageQueryFilter, PagesRepo, PagesWriteRepo, RepoError,
    RestorePageSnapshotParams, SettingsRepo, UpdatePageParams, UpdatePageStatusParams,
};
use crate::cache::CacheTrigger;
use crate::domain::entities::PageRecord;
use crate::domain::{
    slug::{SlugAsyncError, SlugError, generate_unique_slug_async},
    types::PageStatus,
};

#[derive(Debug, Error)]
pub enum AdminPageError {
    #[error("{0}")]
    ConstraintViolation(&'static str),
    #[error(transparent)]
    Render(#[from] RenderError),
    #[error(transparent)]
    Repo(#[from] RepoError),
}

#[derive(Debug, Clone)]
pub struct CreatePageCommand {
    pub title: String,
    pub body_markdown: String,
    pub status: PageStatus,
    pub scheduled_at: Option<OffsetDateTime>,
    pub published_at: Option<OffsetDateTime>,
    pub archived_at: Option<OffsetDateTime>,
}

#[derive(Debug, Clone)]
pub struct UpdatePageContentCommand {
    pub id: Uuid,
    pub slug: String,
    pub title: String,
    pub body_markdown: String,
}

#[derive(Debug, Clone)]
pub struct UpdatePageStatusCommand {
    pub id: Uuid,
    pub status: PageStatus,
    pub scheduled_at: Option<OffsetDateTime>,
    pub published_at: Option<OffsetDateTime>,
    pub archived_at: Option<OffsetDateTime>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AdminPageStatusCounts {
    pub total: u64,
    pub draft: u64,
    pub published: u64,
    pub archived: u64,
    pub error: u64,
}

#[derive(Clone)]
pub struct AdminPageService {
    reader: Arc<dyn PagesRepo>,
    writer: Arc<dyn PagesWriteRepo>,
    jobs: Arc<dyn JobsRepo>,
    audit: AdminAuditService,
    settings: Arc<dyn SettingsRepo>,
    cache_trigger: Option<Arc<CacheTrigger>>,
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
        Ok(page)
    }

    pub async fn list(
        &self,
        status: Option<PageStatus>,
        limit: u32,
        cursor: Option<PageCursor>,
        filter: &PageQueryFilter,
    ) -> Result<CursorPage<PageRecord>, AdminPageError> {
        self.reader
            .list_pages(status, limit, cursor, filter)
            .await
            .map_err(AdminPageError::from)
    }

    pub async fn find_by_slug(&self, slug: &str) -> Result<Option<PageRecord>, AdminPageError> {
        self.reader
            .find_by_slug(slug)
            .await
            .map_err(AdminPageError::from)
    }

    pub async fn find_by_id(&self, id: Uuid) -> Result<Option<PageRecord>, AdminPageError> {
        self.reader
            .find_by_id(id)
            .await
            .map_err(AdminPageError::from)
    }

    pub async fn status_counts(
        &self,
        filter: &PageQueryFilter,
    ) -> Result<AdminPageStatusCounts, AdminPageError> {
        let total_filter = filter.clone();
        let draft_filter = filter.clone();
        let published_filter = filter.clone();
        let archived_filter = filter.clone();
        let error_filter = filter.clone();

        let total_fut = self.reader.count_pages(None, &total_filter);
        let draft_fut = self
            .reader
            .count_pages(Some(PageStatus::Draft), &draft_filter);
        let published_fut = self
            .reader
            .count_pages(Some(PageStatus::Published), &published_filter);
        let archived_fut = self
            .reader
            .count_pages(Some(PageStatus::Archived), &archived_filter);
        let error_fut = self
            .reader
            .count_pages(Some(PageStatus::Error), &error_filter);

        let (total, draft, published, archived, error) =
            tokio::try_join!(total_fut, draft_fut, published_fut, archived_fut, error_fut)?;

        Ok(AdminPageStatusCounts {
            total,
            draft,
            published,
            archived,
            error,
        })
    }

    pub async fn month_counts(
        &self,
        status: Option<PageStatus>,
        filter: &PageQueryFilter,
    ) -> Result<Vec<crate::domain::posts::MonthCount>, AdminPageError> {
        self.reader
            .list_month_counts(status, filter)
            .await
            .map_err(AdminPageError::from)
    }

    pub async fn create_page(
        &self,
        actor: &str,
        command: CreatePageCommand,
    ) -> Result<PageRecord, AdminPageError> {
        ensure_non_empty(&command.title, "title")?;
        ensure_non_empty(&command.body_markdown, "body_markdown")?;

        let CreatePageCommand {
            title,
            body_markdown,
            status,
            scheduled_at,
            published_at,
            archived_at,
        } = command;

        let reader = self.reader.clone();
        let slug = match generate_unique_slug_async(&title, move |candidate| {
            let reader = reader.clone();
            let candidate = candidate.to_string();
            async move {
                reader
                    .find_by_slug(&candidate)
                    .await
                    .map(|existing| existing.is_none())
            }
        })
        .await
        {
            Ok(slug) => slug,
            Err(SlugAsyncError::Slug(err)) => match err {
                SlugError::EmptyInput | SlugError::Unrepresentable { .. } => {
                    return Err(AdminPageError::ConstraintViolation("title"));
                }
                SlugError::Exhausted { .. } => {
                    return Err(AdminPageError::ConstraintViolation("slug"));
                }
            },
            Err(SlugAsyncError::Predicate(err)) => return Err(AdminPageError::Repo(err)),
        };

        let timestamps = normalize_status(status, scheduled_at, published_at, archived_at)?;

        let site_settings = self.settings.load_site_settings().await?;
        let public_site_url = normalize_public_site_url(&site_settings.public_site_url);

        let render_request = RenderRequest::new(
            RenderTarget::PageBody { slug: slug.clone() },
            body_markdown.clone(),
        )
        .with_public_site_url(&public_site_url);
        let render_output = render_service().render(&render_request)?;

        let params = CreatePageParams {
            slug: slug.clone(),
            title: title.clone(),
            body_markdown: body_markdown.clone(),
            rendered_html: render_output.html.clone(),
            status,
            scheduled_at: timestamps.scheduled_at,
            published_at: timestamps.published_at,
            archived_at: timestamps.archived_at,
        };

        let page = self.writer.create_page(params).await?;
        let snapshot = PageSummarySnapshot {
            slug: page.slug.as_str(),
            title: page.title.as_str(),
            status: page.status,
        };
        self.audit
            .record(
                actor,
                "page.create",
                "page",
                Some(&page.id.to_string()),
                Some(&snapshot),
            )
            .await?;
        self.enqueue_render_job(&page).await?;

        // Trigger cache invalidation
        if let Some(trigger) = &self.cache_trigger {
            trigger.page_upserted(page.id, &page.slug).await;
        }

        Ok(page)
    }

    pub async fn update_page(
        &self,
        actor: &str,
        command: UpdatePageContentCommand,
    ) -> Result<PageRecord, AdminPageError> {
        ensure_non_empty(&command.slug, "slug")?;
        ensure_non_empty(&command.title, "title")?;
        ensure_non_empty(&command.body_markdown, "body_markdown")?;

        let site_settings = self.settings.load_site_settings().await?;
        let public_site_url = normalize_public_site_url(&site_settings.public_site_url);

        let render_request = RenderRequest::new(
            RenderTarget::PageBody {
                slug: command.slug.clone(),
            },
            command.body_markdown.clone(),
        )
        .with_public_site_url(&public_site_url);
        let render_output = render_service().render(&render_request)?;

        let params = UpdatePageParams {
            id: command.id,
            slug: command.slug,
            title: command.title,
            body_markdown: command.body_markdown,
            rendered_html: render_output.html,
        };

        let page = self.writer.update_page(params).await?;
        let snapshot = PageSummarySnapshot {
            slug: page.slug.as_str(),
            title: page.title.as_str(),
            status: page.status,
        };
        self.audit
            .record(
                actor,
                "page.update",
                "page",
                Some(&page.id.to_string()),
                Some(&snapshot),
            )
            .await?;
        self.enqueue_render_job(&page).await?;

        // Trigger cache invalidation
        if let Some(trigger) = &self.cache_trigger {
            trigger.page_upserted(page.id, &page.slug).await;
        }

        Ok(page)
    }

    pub async fn update_status(
        &self,
        actor: &str,
        command: UpdatePageStatusCommand,
    ) -> Result<PageRecord, AdminPageError> {
        if command.status == PageStatus::Published {
            let publish_at = command.published_at.unwrap_or_else(OffsetDateTime::now_utc);

            let scheduled = self
                .writer
                .schedule_page_publication(command.id, publish_at)
                .await?;

            let job_id =
                enqueue_publish_page_job(self.jobs.as_ref(), scheduled.slug.clone(), publish_at)
                    .await?;

            let mut page = scheduled;
            if publish_at <= OffsetDateTime::now_utc() {
                wait_for_job_completion(self.jobs.as_ref(), &job_id, PUBLISH_JOB_WAIT_TIMEOUT)
                    .await?;

                page = self.reader.find_by_slug(&page.slug).await?.ok_or_else(|| {
                    AdminPageError::Repo(RepoError::from_persistence(
                        "page missing after publish job",
                    ))
                })?;
            }

            self.record_status_audit(actor, &page).await?;

            // Trigger cache invalidation
            if let Some(trigger) = &self.cache_trigger {
                trigger.page_upserted(page.id, &page.slug).await;
            }

            Ok(page)
        } else {
            let normalized = normalize_status(
                command.status,
                command.scheduled_at,
                command.published_at,
                command.archived_at,
            )?;

            let params = UpdatePageStatusParams {
                id: command.id,
                status: command.status,
                scheduled_at: normalized.scheduled_at,
                published_at: normalized.published_at,
                archived_at: normalized.archived_at,
            };

            let page = self.writer.update_page_status(params).await?;
            self.record_status_audit(actor, &page).await?;

            // Trigger cache invalidation
            if let Some(trigger) = &self.cache_trigger {
                trigger.page_upserted(page.id, &page.slug).await;
            }

            Ok(page)
        }
    }

    pub async fn delete_page(&self, actor: &str, id: Uuid, slug: &str) -> Result<(), AdminPageError> {
        self.writer.delete_page(id).await?;
        self.audit
            .record(
                actor,
                "page.delete",
                "page",
                Some(&id.to_string()),
                Option::<&PageSummarySnapshot<'_>>::None,
            )
            .await?;

        // Trigger cache invalidation
        if let Some(trigger) = &self.cache_trigger {
            trigger.page_deleted(id, slug).await;
        }

        Ok(())
    }

    async fn enqueue_render_job(&self, page: &PageRecord) -> Result<(), AdminPageError> {
        enqueue_render_page_job(
            self.jobs.as_ref(),
            page.slug.clone(),
            page.body_markdown.clone(),
            None,
        )
        .await?;

        Ok(())
    }

    async fn record_status_audit(
        &self,
        actor: &str,
        page: &PageRecord,
    ) -> Result<(), AdminPageError> {
        let snapshot = PageSummarySnapshot {
            slug: page.slug.as_str(),
            title: page.title.as_str(),
            status: page.status,
        };
        self.audit
            .record(
                actor,
                "page.status",
                "page",
                Some(&page.id.to_string()),
                Some(&snapshot),
            )
            .await?;
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct PageSummarySnapshot<'a> {
    pub slug: &'a str,
    pub title: &'a str,
    pub status: PageStatus,
}

struct StatusTimestamps {
    scheduled_at: Option<OffsetDateTime>,
    published_at: Option<OffsetDateTime>,
    archived_at: Option<OffsetDateTime>,
}

fn normalize_status(
    status: PageStatus,
    scheduled_at: Option<OffsetDateTime>,
    published_at: Option<OffsetDateTime>,
    archived_at: Option<OffsetDateTime>,
) -> Result<StatusTimestamps, AdminPageError> {
    match status {
        PageStatus::Published => Ok(StatusTimestamps {
            scheduled_at: None,
            published_at: Some(published_at.unwrap_or_else(OffsetDateTime::now_utc)),
            archived_at: None,
        }),
        PageStatus::Archived => Ok(StatusTimestamps {
            scheduled_at: None,
            published_at,
            archived_at: Some(archived_at.unwrap_or_else(OffsetDateTime::now_utc)),
        }),
        PageStatus::Draft => Ok(StatusTimestamps {
            scheduled_at,
            published_at: None,
            archived_at: None,
        }),
        PageStatus::Error => Ok(StatusTimestamps {
            scheduled_at,
            published_at,
            archived_at,
        }),
    }
}

fn ensure_non_empty(value: &str, field: &'static str) -> Result<(), AdminPageError> {
    if value.trim().is_empty() {
        return Err(AdminPageError::ConstraintViolation(field));
    }
    Ok(())
}

fn normalize_public_site_url(url: &str) -> String {
    let trimmed = url.trim();
    if trimmed.is_empty() {
        return String::new();
    }

    let without_trailing = trimmed.trim_end_matches('/');
    format!("{without_trailing}/")
}
