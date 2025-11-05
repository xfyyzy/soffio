use time::OffsetDateTime;
use uuid::Uuid;

use crate::domain::entities::{PostRecord, PostSectionRecord};
use crate::domain::types::PostStatus;

#[derive(sqlx::FromRow)]
pub(crate) struct PostRow {
    pub(crate) id: Uuid,
    pub(crate) slug: String,
    pub(crate) title: String,
    pub(crate) excerpt: String,
    pub(crate) body_markdown: String,
    pub(crate) status: PostStatus,
    pub(crate) pinned: bool,
    pub(crate) scheduled_at: Option<OffsetDateTime>,
    pub(crate) published_at: Option<OffsetDateTime>,
    pub(crate) archived_at: Option<OffsetDateTime>,
    pub(crate) summary_markdown: Option<String>,
    pub(crate) summary_html: Option<String>,
    pub(crate) created_at: OffsetDateTime,
    pub(crate) updated_at: OffsetDateTime,
    pub(crate) primary_time: OffsetDateTime,
}

impl From<PostRow> for PostRecord {
    fn from(row: PostRow) -> Self {
        Self {
            id: row.id,
            slug: row.slug,
            title: row.title,
            excerpt: row.excerpt,
            body_markdown: row.body_markdown,
            status: row.status,
            pinned: row.pinned,
            scheduled_at: row.scheduled_at,
            published_at: row.published_at,
            archived_at: row.archived_at,
            summary_markdown: row.summary_markdown,
            summary_html: row.summary_html,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

#[derive(sqlx::FromRow)]
pub(crate) struct PostSectionRow {
    pub(crate) id: Uuid,
    pub(crate) post_id: Uuid,
    pub(crate) position: i32,
    pub(crate) level: i16,
    pub(crate) parent_id: Option<Uuid>,
    pub(crate) heading_html: String,
    pub(crate) heading_text: String,
    pub(crate) body_html: String,
    pub(crate) contains_code: bool,
    pub(crate) contains_math: bool,
    pub(crate) contains_mermaid: bool,
    pub(crate) anchor_slug: String,
    pub(crate) created_at: OffsetDateTime,
}

impl From<PostSectionRow> for PostSectionRecord {
    fn from(row: PostSectionRow) -> Self {
        Self {
            id: row.id,
            post_id: row.post_id,
            position: row.position,
            level: row.level,
            parent_id: row.parent_id,
            heading_html: row.heading_html,
            heading_text: row.heading_text,
            body_html: row.body_html,
            contains_code: row.contains_code,
            contains_math: row.contains_math,
            contains_mermaid: row.contains_mermaid,
            anchor_slug: row.anchor_slug,
            created_at: row.created_at,
        }
    }
}

pub struct PersistedPostSection<'a> {
    pub id: Uuid,
    pub parent_id: Option<Uuid>,
    pub position: i32,
    pub level: i16,
    pub heading_html: &'a str,
    pub heading_text: &'a str,
    pub body_html: &'a str,
    pub contains_code: bool,
    pub contains_math: bool,
    pub contains_mermaid: bool,
    pub anchor_slug: &'a str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PersistedPostSectionOwned {
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

impl<'a> From<PersistedPostSection<'a>> for PersistedPostSectionOwned {
    fn from(value: PersistedPostSection<'a>) -> Self {
        Self {
            id: value.id,
            parent_id: value.parent_id,
            position: value.position,
            level: value.level,
            heading_html: value.heading_html.to_string(),
            heading_text: value.heading_text.to_string(),
            body_html: value.body_html.to_string(),
            contains_code: value.contains_code,
            contains_math: value.contains_math,
            contains_mermaid: value.contains_mermaid,
            anchor_slug: value.anchor_slug.to_string(),
        }
    }
}

impl<'a> From<&PersistedPostSection<'a>> for PersistedPostSectionOwned {
    fn from(value: &PersistedPostSection<'a>) -> Self {
        PersistedPostSectionOwned {
            id: value.id,
            parent_id: value.parent_id,
            position: value.position,
            level: value.level,
            heading_html: value.heading_html.to_string(),
            heading_text: value.heading_text.to_string(),
            body_html: value.body_html.to_string(),
            contains_code: value.contains_code,
            contains_math: value.contains_math,
            contains_mermaid: value.contains_mermaid,
            anchor_slug: value.anchor_slug.to_string(),
        }
    }
}
