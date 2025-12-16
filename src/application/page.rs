use std::sync::Arc;

use axum::http::StatusCode;
use uuid::Uuid;

use crate::application::error::HttpError;
use crate::application::repos::{PagesRepo, RepoError};
use crate::domain::types::PageStatus;
use crate::presentation::views::PageView;

const SOURCE: &str = "application::page::PageService";

#[derive(Clone)]
pub struct PageService {
    pages: Arc<dyn PagesRepo>,
}

impl PageService {
    pub fn new(pages: Arc<dyn PagesRepo>) -> Self {
        Self { pages }
    }

    pub async fn page_view(&self, slug: &str) -> Result<Option<PageView>, HttpError> {
        // Record dependencies for L1 cache invalidation
        crate::cache::deps::record(crate::cache::EntityKey::SiteSettings);
        crate::cache::deps::record(crate::cache::EntityKey::PageSlug(slug.to_string()));

        let record = self
            .pages
            .find_by_slug(slug)
            .await
            .map_err(|err| repo_failure("find_by_slug", err))?;

        let Some(record) = record else {
            return Ok(None);
        };

        if record.status != PageStatus::Published || record.published_at.is_none() {
            return Ok(None);
        }

        let rendered_html = record.rendered_html;
        let (contains_code, contains_math, contains_mermaid) = render_feature_flags(&rendered_html);

        Ok(Some(PageView {
            title: record.title,
            content_html: rendered_html,
            contains_code,
            contains_math,
            contains_mermaid,
        }))
    }

    pub async fn page_preview(&self, id: Uuid) -> Result<Option<PageView>, HttpError> {
        let record = self
            .pages
            .find_by_id(id)
            .await
            .map_err(|err| repo_failure("find_by_id", err))?;

        Ok(record.map(|record| {
            let (contains_code, contains_math, contains_mermaid) =
                render_feature_flags(&record.rendered_html);
            PageView {
                title: record.title,
                content_html: record.rendered_html,
                contains_code,
                contains_math,
                contains_mermaid,
            }
        }))
    }
}

fn repo_failure(operation: &'static str, err: RepoError) -> HttpError {
    HttpError::new(
        SOURCE,
        StatusCode::INTERNAL_SERVER_ERROR,
        "Failed to load page content",
        format!("{operation} failed: {err}"),
    )
}

fn render_feature_flags(html: &str) -> (bool, bool, bool) {
    let contains_code = html.contains("syntax-") || html.contains("<pre") || html.contains("<code");
    let contains_math = html.contains("data-math-style");
    let contains_mermaid = html.contains("data-role=\"diagram-mermaid\"");
    (contains_code, contains_math, contains_mermaid)
}
