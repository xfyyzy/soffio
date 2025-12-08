use std::collections::{BTreeSet, HashMap};

use time::OffsetDateTime;
use uuid::Uuid;

use crate::application::jobs::{
    PUBLISH_JOB_WAIT_TIMEOUT, enqueue_publish_post_job, wait_for_job_completion,
};
use crate::application::render::enqueue_render_post_job;
use crate::application::repos::{
    CreatePostParams, RepoError, UpdatePostParams, UpdatePostPinnedParams, UpdatePostStatusParams,
};
use crate::domain::entities::PostRecord;
use crate::domain::slug::{SlugAsyncError, SlugError, generate_unique_slug_async};
use crate::domain::types::PostStatus;

use super::service::AdminPostService;
use super::types::{
    AdminPostError, CreatePostCommand, PostSummarySnapshot, PostTagsSnapshot,
    UpdatePostContentCommand, UpdatePostStatusCommand, ensure_non_empty, normalize_status,
};

impl AdminPostService {
    pub async fn create_post(
        &self,
        actor: &str,
        command: CreatePostCommand,
    ) -> Result<PostRecord, AdminPostError> {
        ensure_non_empty(&command.title, "title")?;
        ensure_non_empty(&command.excerpt, "excerpt")?;
        ensure_non_empty(&command.body_markdown, "body_markdown")?;

        let reader = self.reader.clone();
        let slug = match generate_unique_slug_async(&command.title, move |candidate| {
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
                    return Err(AdminPostError::ConstraintViolation("title"));
                }
                SlugError::Exhausted { .. } => {
                    return Err(AdminPostError::ConstraintViolation("slug"));
                }
            },
            Err(SlugAsyncError::Predicate(err)) => return Err(AdminPostError::Repo(err)),
        };

        let timestamps = normalize_status(
            command.status,
            command.scheduled_at,
            command.published_at,
            command.archived_at,
        )?;

        let params = CreatePostParams {
            slug,
            title: command.title,
            excerpt: command.excerpt,
            body_markdown: command.body_markdown,
            status: command.status,
            pinned: command.pinned,
            scheduled_at: timestamps.scheduled_at,
            published_at: timestamps.published_at,
            archived_at: timestamps.archived_at,
            summary_markdown: command.summary_markdown,
            summary_html: None,
        };

        let post = self.writer.create_post(params).await?;

        let snapshot = PostSummarySnapshot {
            slug: post.slug.as_str(),
            title: post.title.as_str(),
            status: post.status,
        };
        self.audit
            .record(
                actor,
                "post.create",
                "post",
                Some(&post.id.to_string()),
                Some(&snapshot),
            )
            .await?;

        self.enqueue_render_jobs(&post).await?;

        Ok(post)
    }

    pub async fn update_post(
        &self,
        actor: &str,
        command: UpdatePostContentCommand,
    ) -> Result<PostRecord, AdminPostError> {
        ensure_non_empty(&command.slug, "slug")?;
        ensure_non_empty(&command.title, "title")?;
        ensure_non_empty(&command.excerpt, "excerpt")?;
        ensure_non_empty(&command.body_markdown, "body_markdown")?;

        let params = UpdatePostParams {
            id: command.id,
            slug: command.slug,
            title: command.title,
            excerpt: command.excerpt,
            body_markdown: command.body_markdown,
            pinned: command.pinned,
            summary_markdown: command.summary_markdown,
            summary_html: None,
        };

        let post = self.writer.update_post(params).await?;

        let snapshot = PostSummarySnapshot {
            slug: post.slug.as_str(),
            title: post.title.as_str(),
            status: post.status,
        };
        self.audit
            .record(
                actor,
                "post.update",
                "post",
                Some(&post.id.to_string()),
                Some(&snapshot),
            )
            .await?;

        self.enqueue_render_jobs(&post).await?;

        Ok(post)
    }

    pub async fn update_status(
        &self,
        actor: &str,
        command: UpdatePostStatusCommand,
    ) -> Result<PostRecord, AdminPostError> {
        if command.status == PostStatus::Published {
            let publish_at = command.published_at.unwrap_or_else(OffsetDateTime::now_utc);

            let scheduled = self
                .writer
                .schedule_post_publication(command.id, publish_at)
                .await?;

            let job_id =
                enqueue_publish_post_job(self.jobs.as_ref(), scheduled.slug.clone(), publish_at)
                    .await?;

            let mut post = scheduled;
            if publish_at <= OffsetDateTime::now_utc() {
                wait_for_job_completion(self.jobs.as_ref(), &job_id, PUBLISH_JOB_WAIT_TIMEOUT)
                    .await?;

                post = self.reader.find_by_slug(&post.slug).await?.ok_or_else(|| {
                    AdminPostError::Repo(RepoError::from_persistence(
                        "post missing after publish job",
                    ))
                })?;
            }

            self.record_status_audit(actor, &post).await?;
            Ok(post)
        } else {
            let normalized = normalize_status(
                command.status,
                command.scheduled_at,
                command.published_at,
                command.archived_at,
            )?;

            let params = UpdatePostStatusParams {
                id: command.id,
                status: command.status,
                scheduled_at: normalized.scheduled_at,
                published_at: normalized.published_at,
                archived_at: normalized.archived_at,
            };

            let post = self.writer.update_post_status(params).await?;
            self.record_status_audit(actor, &post).await?;
            Ok(post)
        }
    }

    pub async fn delete_post(&self, actor: &str, id: Uuid) -> Result<(), AdminPostError> {
        self.writer.delete_post(id).await?;
        self.audit
            .record(
                actor,
                "post.delete",
                "post",
                Some(&id.to_string()),
                Option::<&PostSummarySnapshot<'_>>::None,
            )
            .await?;

        Ok(())
    }

    pub async fn update_pin_state(
        &self,
        actor: &str,
        id: Uuid,
        pinned: bool,
    ) -> Result<PostRecord, AdminPostError> {
        let post = self
            .writer
            .update_post_pinned(UpdatePostPinnedParams { id, pinned })
            .await?;

        let snapshot = PostSummarySnapshot {
            slug: post.slug.as_str(),
            title: post.title.as_str(),
            status: post.status,
        };

        let action = if post.pinned {
            "post.pin"
        } else {
            "post.unpin"
        };
        self.audit
            .record(
                actor,
                action,
                "post",
                Some(&post.id.to_string()),
                Some(&snapshot),
            )
            .await?;

        Ok(post)
    }

    pub async fn replace_tags(
        &self,
        actor: &str,
        post: &PostRecord,
        tag_ids: &[Uuid],
    ) -> Result<(), AdminPostError> {
        let mut seen = BTreeSet::new();
        let mut normalized = Vec::new();
        for id in tag_ids {
            if seen.insert(*id) {
                normalized.push(*id);
            }
        }

        self.writer.replace_post_tags(post.id, &normalized).await?;

        let tag_slugs = self.resolve_tag_slugs(&normalized).await?;

        let snapshot = PostTagsSnapshot {
            slug: post.slug.as_str(),
            title: post.title.as_str(),
            tags: tag_slugs.as_slice(),
        };

        self.audit
            .record(
                actor,
                "post.tags",
                "post",
                Some(&post.id.to_string()),
                Some(&snapshot),
            )
            .await?;

        Ok(())
    }

    async fn resolve_tag_slugs(&self, tag_ids: &[Uuid]) -> Result<Vec<String>, AdminPostError> {
        if tag_ids.is_empty() {
            return Ok(Vec::new());
        }

        let records = self.tags.list_all().await?;
        let tag_lookup: HashMap<Uuid, String> =
            records.into_iter().map(|tag| (tag.id, tag.slug)).collect();

        let mut slugs = Vec::new();
        for id in tag_ids {
            if let Some(slug) = tag_lookup.get(id) {
                slugs.push(slug.clone());
            }
        }

        slugs.sort();
        slugs.dedup();

        Ok(slugs)
    }

    async fn enqueue_render_jobs(&self, post: &PostRecord) -> Result<(), AdminPostError> {
        enqueue_render_post_job(self.jobs.as_ref(), post.slug.clone(), None).await?;

        Ok(())
    }

    async fn record_status_audit(
        &self,
        actor: &str,
        post: &PostRecord,
    ) -> Result<(), AdminPostError> {
        let snapshot = PostSummarySnapshot {
            slug: post.slug.as_str(),
            title: post.title.as_str(),
            status: post.status,
        };
        self.audit
            .record(
                actor,
                "post.status",
                "post",
                Some(&post.id.to_string()),
                Some(&snapshot),
            )
            .await?;
        Ok(())
    }
}
