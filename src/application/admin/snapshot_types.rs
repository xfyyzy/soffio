use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::domain::types::{PageStatus, PostStatus};
use crate::domain::{
    entities::{PageRecord, PostRecord, PostSectionRecord},
    snapshots::{SnapshotError, Snapshotable},
    types::SnapshotEntityType,
};

#[derive(Debug, Clone)]
pub struct PostSnapshotSource {
    pub post: PostRecord,
    pub tags: Vec<Uuid>,
    pub sections: Vec<PostSectionRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostSectionSnapshot {
    pub id: Uuid,
    pub parent_id: Option<Uuid>,
    pub position: i32,
    pub level: i16,
    pub heading_html: String,
    pub heading_text: String,
    pub body_html: String,
    pub contains_code: bool,
    pub contains_math: bool,
    pub contains_mermaid: bool,
    pub anchor_slug: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostSnapshotPayload {
    pub slug: String,
    pub title: String,
    pub excerpt: String,
    pub body_markdown: String,
    pub summary_markdown: Option<String>,
    pub summary_html: Option<String>,
    pub status: PostStatus,
    pub pinned: bool,
    pub scheduled_at: Option<OffsetDateTime>,
    pub published_at: Option<OffsetDateTime>,
    pub archived_at: Option<OffsetDateTime>,
    pub tags: Vec<Uuid>,
    pub sections: Vec<PostSectionSnapshot>,
}

impl Snapshotable for PostSnapshotSource {
    type Id = Uuid;
    type Payload = PostSnapshotPayload;

    const ENTITY_TYPE: SnapshotEntityType = SnapshotEntityType::Post;

    fn id(&self) -> &Self::Id {
        &self.post.id
    }

    fn to_snapshot(&self) -> Result<Self::Payload, SnapshotError> {
        if self.post.slug.trim().is_empty() {
            return Err(SnapshotError::Validation("slug cannot be empty".into()));
        }
        if self.post.title.trim().is_empty() {
            return Err(SnapshotError::Validation("title cannot be empty".into()));
        }
        if self.post.excerpt.trim().is_empty() {
            return Err(SnapshotError::Validation("excerpt cannot be empty".into()));
        }
        if self.post.body_markdown.trim().is_empty() {
            return Err(SnapshotError::Validation(
                "body_markdown cannot be empty".into(),
            ));
        }

        let sections = self
            .sections
            .iter()
            .map(|s| PostSectionSnapshot {
                id: s.id,
                parent_id: s.parent_id,
                position: s.position,
                level: s.level,
                heading_html: s.heading_html.clone(),
                heading_text: s.heading_text.clone(),
                body_html: s.body_html.clone(),
                contains_code: s.contains_code,
                contains_math: s.contains_math,
                contains_mermaid: s.contains_mermaid,
                anchor_slug: s.anchor_slug.clone(),
            })
            .collect();

        Ok(PostSnapshotPayload {
            slug: self.post.slug.clone(),
            title: self.post.title.clone(),
            excerpt: self.post.excerpt.clone(),
            body_markdown: self.post.body_markdown.clone(),
            summary_markdown: self.post.summary_markdown.clone(),
            summary_html: self.post.summary_html.clone(),
            status: self.post.status,
            pinned: self.post.pinned,
            scheduled_at: self.post.scheduled_at,
            published_at: self.post.published_at,
            archived_at: self.post.archived_at,
            tags: self.tags.clone(),
            sections,
        })
    }

    fn validate_snapshot(payload: &Self::Payload) -> Result<(), SnapshotError> {
        if payload.slug.trim().is_empty() {
            return Err(SnapshotError::Validation("slug cannot be empty".into()));
        }
        if payload.title.trim().is_empty() {
            return Err(SnapshotError::Validation("title cannot be empty".into()));
        }
        if payload.body_markdown.trim().is_empty() {
            return Err(SnapshotError::Validation(
                "body_markdown cannot be empty".into(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageSnapshotPayload {
    pub slug: String,
    pub title: String,
    pub body_markdown: String,
    pub rendered_html: String,
    pub status: PageStatus,
    pub scheduled_at: Option<OffsetDateTime>,
    pub published_at: Option<OffsetDateTime>,
    pub archived_at: Option<OffsetDateTime>,
}

#[derive(Debug, Clone)]
pub struct PageSnapshotSource {
    pub page: PageRecord,
}

impl Snapshotable for PageSnapshotSource {
    type Id = Uuid;
    type Payload = PageSnapshotPayload;

    const ENTITY_TYPE: SnapshotEntityType = SnapshotEntityType::Page;

    fn id(&self) -> &Self::Id {
        &self.page.id
    }

    fn to_snapshot(&self) -> Result<Self::Payload, SnapshotError> {
        if self.page.slug.trim().is_empty() {
            return Err(SnapshotError::Validation("slug cannot be empty".into()));
        }
        if self.page.title.trim().is_empty() {
            return Err(SnapshotError::Validation("title cannot be empty".into()));
        }
        if self.page.body_markdown.trim().is_empty() {
            return Err(SnapshotError::Validation(
                "body_markdown cannot be empty".into(),
            ));
        }

        Ok(PageSnapshotPayload {
            slug: self.page.slug.clone(),
            title: self.page.title.clone(),
            body_markdown: self.page.body_markdown.clone(),
            rendered_html: self.page.rendered_html.clone(),
            status: self.page.status,
            scheduled_at: self.page.scheduled_at,
            published_at: self.page.published_at,
            archived_at: self.page.archived_at,
        })
    }

    fn validate_snapshot(payload: &Self::Payload) -> Result<(), SnapshotError> {
        if payload.slug.trim().is_empty() {
            return Err(SnapshotError::Validation("slug cannot be empty".into()));
        }
        if payload.title.trim().is_empty() {
            return Err(SnapshotError::Validation("title cannot be empty".into()));
        }
        if payload.body_markdown.trim().is_empty() {
            return Err(SnapshotError::Validation(
                "body_markdown cannot be empty".into(),
            ));
        }
        if payload.rendered_html.trim().is_empty() {
            return Err(SnapshotError::Validation(
                "rendered_html cannot be empty".into(),
            ));
        }
        Ok(())
    }
}
