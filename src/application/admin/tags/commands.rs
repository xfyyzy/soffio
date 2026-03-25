use serde::Serialize;
use uuid::Uuid;

use crate::application::repos::{CreateTagParams, RepoError, UpdateTagParams};
use crate::domain::entities::TagRecord;
use crate::domain::slug::{SlugAsyncError, SlugError, generate_unique_slug_async};

use super::service::AdminTagService;
use super::types::{
    AdminTagError, CreateTagCommand, UpdateTagCommand, ensure_non_empty, normalize_optional_text,
};

impl AdminTagService {
    pub async fn create_tag(
        &self,
        actor: &str,
        command: CreateTagCommand,
    ) -> Result<TagRecord, AdminTagError> {
        ensure_non_empty(&command.name, "name")?;

        let CreateTagCommand {
            name,
            description,
            pinned,
        } = command;

        let name = name.trim().to_string();
        ensure_non_empty(&name, "name")?;
        let description = normalize_optional_text(description);

        let reader = self.reader.clone();
        let slug = match generate_unique_slug_async(&name, move |candidate| {
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
                    return Err(AdminTagError::ConstraintViolation("name"));
                }
                SlugError::Exhausted { .. } => {
                    return Err(AdminTagError::ConstraintViolation("slug"));
                }
            },
            Err(SlugAsyncError::Predicate(err)) => return Err(AdminTagError::Repo(err)),
        };

        let params = CreateTagParams {
            slug,
            name,
            description,
            pinned,
        };

        let tag = self.writer.create_tag(params).await?;
        let snapshot = TagSnapshot {
            slug: tag.slug.as_str(),
            name: tag.name.as_str(),
        };
        self.audit
            .record(
                actor,
                "tag.create",
                "tag",
                Some(&tag.id.to_string()),
                Some(&snapshot),
            )
            .await?;

        if let Some(trigger) = &self.cache_trigger {
            trigger.tags_changed().await;
        }

        Ok(tag)
    }

    pub async fn update_tag(
        &self,
        actor: &str,
        command: UpdateTagCommand,
    ) -> Result<TagRecord, AdminTagError> {
        ensure_non_empty(&command.name, "name")?;

        let UpdateTagCommand {
            id,
            name,
            description,
            pinned,
        } = command;

        let name = name.trim().to_string();
        ensure_non_empty(&name, "name")?;
        let description = normalize_optional_text(description);

        let existing = self
            .reader
            .find_by_id(id)
            .await?
            .ok_or_else(|| RepoError::from_persistence("tag not found"))?;

        let params = UpdateTagParams {
            id,
            slug: existing.slug.clone(),
            name,
            description,
            pinned,
        };

        let tag = self.writer.update_tag(params).await?;
        let snapshot = TagSnapshot {
            slug: tag.slug.as_str(),
            name: tag.name.as_str(),
        };
        self.audit
            .record(
                actor,
                "tag.update",
                "tag",
                Some(&tag.id.to_string()),
                Some(&snapshot),
            )
            .await?;

        if let Some(trigger) = &self.cache_trigger {
            trigger.tags_changed().await;
        }

        Ok(tag)
    }

    pub async fn update_tag_pinned(
        &self,
        actor: &str,
        id: Uuid,
        pinned: bool,
    ) -> Result<TagRecord, AdminTagError> {
        let existing = self
            .reader
            .find_by_id(id)
            .await?
            .ok_or_else(|| AdminTagError::Repo(RepoError::from_persistence("tag not found")))?;

        let command = UpdateTagCommand {
            id,
            name: existing.name.clone(),
            description: existing.description.clone(),
            pinned,
        };

        self.update_tag(actor, command).await
    }

    pub async fn delete_tag(&self, actor: &str, id: Uuid) -> Result<(), AdminTagError> {
        let usage = self.reader.count_usage(id).await?;
        if usage > 0 {
            return Err(AdminTagError::InUse { count: usage });
        }

        self.writer.delete_tag(id).await?;
        self.audit
            .record(
                actor,
                "tag.delete",
                "tag",
                Some(&id.to_string()),
                Option::<&TagSnapshot<'_>>::None,
            )
            .await?;

        if let Some(trigger) = &self.cache_trigger {
            trigger.tags_changed().await;
        }

        Ok(())
    }
}

#[derive(Debug, Serialize)]
struct TagSnapshot<'a> {
    slug: &'a str,
    name: &'a str,
}
