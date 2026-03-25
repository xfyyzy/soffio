use time::OffsetDateTime;
use uuid::Uuid;

use crate::application::jobs::{
    PUBLISH_JOB_WAIT_TIMEOUT, enqueue_publish_page_job, wait_for_job_completion,
};
use crate::application::render::{
    RenderRequest, RenderService, RenderTarget, enqueue_render_page_job, render_service,
};
use crate::application::repos::{
    CreatePageParams, RepoError, UpdatePageParams, UpdatePageStatusParams,
};
use crate::domain::entities::PageRecord;
use crate::domain::slug::{SlugAsyncError, SlugError, generate_unique_slug_async};
use crate::domain::types::PageStatus;

use super::service::AdminPageService;
use super::types::{
    AdminPageError, CreatePageCommand, PageSummarySnapshot, UpdatePageContentCommand,
    UpdatePageStatusCommand, ensure_non_empty, normalize_public_site_url, normalize_status,
};

impl AdminPageService {
    pub async fn create_page(
        &self,
        actor: &str,
        command: CreatePageCommand,
    ) -> Result<PageRecord, AdminPageError> {
        ensure_non_empty(&command.title, "title")?;
        ensure_non_empty(&command.body_markdown, "body_markdown")?;

        let CreatePageCommand {
            slug,
            title,
            body_markdown,
            status,
            scheduled_at,
            published_at,
            archived_at,
        } = command;

        let reader = self.reader.clone();
        let slug_is_custom = slug.is_some();
        let slug_source = slug.as_deref().unwrap_or(title.as_str());
        let slug = match generate_unique_slug_async(slug_source, move |candidate| {
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
                    return Err(AdminPageError::ConstraintViolation(if slug_is_custom {
                        "slug"
                    } else {
                        "title"
                    }));
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

        let previous_slug = self
            .reader
            .find_by_id(command.id)
            .await?
            .map(|page| page.slug)
            .ok_or_else(|| RepoError::from_persistence("page not found"))?;

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
            let previous_slug = (previous_slug != page.slug).then_some(previous_slug.as_str());
            trigger
                .page_upserted_with_previous_slug(page.id, &page.slug, previous_slug)
                .await;
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

    pub async fn publish_scheduled_by_slug(
        &self,
        slug: &str,
    ) -> Result<PageRecord, AdminPageError> {
        let Some(page) = self.reader.find_by_slug(slug).await? else {
            return Err(AdminPageError::Repo(RepoError::NotFound));
        };

        let publish_at = page.scheduled_at.unwrap_or_else(OffsetDateTime::now_utc);
        let params = UpdatePageStatusParams {
            id: page.id,
            status: PageStatus::Published,
            scheduled_at: None,
            published_at: Some(publish_at),
            archived_at: None,
        };

        let page = self.writer.update_page_status(params).await?;
        self.record_status_audit("system", &page).await?;

        // Trigger cache invalidation
        if let Some(trigger) = &self.cache_trigger {
            trigger.page_upserted(page.id, &page.slug).await;
        }

        Ok(page)
    }

    pub async fn delete_page(
        &self,
        actor: &str,
        id: Uuid,
        slug: &str,
    ) -> Result<(), AdminPageError> {
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
