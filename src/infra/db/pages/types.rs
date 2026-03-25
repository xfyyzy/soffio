use time::OffsetDateTime;
use uuid::Uuid;

use crate::domain::{entities::PageRecord, types::PageStatus};

pub(super) const PAGE_PRIMARY_TIME_EXPR: &str = "CASE \
    WHEN status = 'published'::page_status THEN \
        COALESCE(published_at, updated_at, created_at) \
    ELSE \
        COALESCE(updated_at, created_at) \
END";

#[derive(sqlx::FromRow)]
pub(super) struct PageRow {
    pub(super) id: Uuid,
    pub(super) slug: String,
    pub(super) title: String,
    pub(super) body_markdown: String,
    pub(super) rendered_html: String,
    pub(super) status: PageStatus,
    pub(super) scheduled_at: Option<OffsetDateTime>,
    pub(super) published_at: Option<OffsetDateTime>,
    pub(super) archived_at: Option<OffsetDateTime>,
    pub(super) created_at: OffsetDateTime,
    pub(super) updated_at: OffsetDateTime,
}

impl From<PageRow> for PageRecord {
    fn from(row: PageRow) -> Self {
        Self {
            id: row.id,
            slug: row.slug,
            title: row.title,
            body_markdown: row.body_markdown,
            rendered_html: row.rendered_html,
            status: row.status,
            scheduled_at: row.scheduled_at,
            published_at: row.published_at,
            archived_at: row.archived_at,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

#[derive(sqlx::FromRow)]
pub(super) struct PageListRow {
    pub(super) id: Uuid,
    pub(super) slug: String,
    pub(super) title: String,
    pub(super) body_markdown: String,
    pub(super) rendered_html: String,
    pub(super) status: PageStatus,
    pub(super) scheduled_at: Option<OffsetDateTime>,
    pub(super) published_at: Option<OffsetDateTime>,
    pub(super) archived_at: Option<OffsetDateTime>,
    pub(super) created_at: OffsetDateTime,
    pub(super) updated_at: OffsetDateTime,
    pub(super) primary_time: OffsetDateTime,
}

impl From<PageListRow> for PageRecord {
    fn from(row: PageListRow) -> Self {
        Self {
            id: row.id,
            slug: row.slug,
            title: row.title,
            body_markdown: row.body_markdown,
            rendered_html: row.rendered_html,
            status: row.status,
            scheduled_at: row.scheduled_at,
            published_at: row.published_at,
            archived_at: row.archived_at,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}
