use std::sync::Arc;

use axum::http::StatusCode;
use uuid::Uuid;

use crate::application::error::HttpError;
use crate::application::repos::{PagesRepo, RepoError};
use crate::cache::L0Store;
use crate::domain::types::PageStatus;
use crate::presentation::views::PageView;

const SOURCE: &str = "application::page::PageService";

#[derive(Clone)]
pub struct PageService {
    pages: Arc<dyn PagesRepo>,
    cache: Option<Arc<L0Store>>,
}

impl PageService {
    pub fn new(pages: Arc<dyn PagesRepo>, cache: Option<Arc<L0Store>>) -> Self {
        Self { pages, cache }
    }

    pub async fn page_view(&self, slug: &str) -> Result<Option<PageView>, HttpError> {
        // Record dependencies for L1 cache invalidation
        crate::cache::deps::record(crate::cache::EntityKey::SiteSettings);
        crate::cache::deps::record(crate::cache::EntityKey::PageSlug(slug.to_string()));

        let record = if let Some(cache) = &self.cache {
            if let Some(cached) = cache.get_page_by_slug(slug) {
                Some(cached)
            } else {
                let fetched = self
                    .pages
                    .find_by_slug(slug)
                    .await
                    .map_err(|err| repo_failure("find_by_slug", err))?;
                if let Some(page) = fetched.clone() {
                    cache.set_page(page);
                }
                fetched
            }
        } else {
            self.pages
                .find_by_slug(slug)
                .await
                .map_err(|err| repo_failure("find_by_slug", err))?
        };

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

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };

    use async_trait::async_trait;
    use time::OffsetDateTime;

    use crate::application::pagination::{CursorPage, PageCursor};
    use crate::application::repos::{PageQueryFilter, RepoError};
    use crate::cache::CacheConfig;
    use crate::domain::entities::PageRecord;
    use crate::domain::posts::MonthCount;

    struct StubPagesRepo {
        calls: Arc<AtomicUsize>,
        page: PageRecord,
    }

    #[async_trait]
    impl PagesRepo for StubPagesRepo {
        async fn list_pages(
            &self,
            _status: Option<PageStatus>,
            _limit: u32,
            _cursor: Option<PageCursor>,
            _filter: &PageQueryFilter,
        ) -> Result<CursorPage<PageRecord>, RepoError> {
            Ok(CursorPage::empty())
        }

        async fn find_by_slug(&self, slug: &str) -> Result<Option<PageRecord>, RepoError> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            if slug == self.page.slug {
                Ok(Some(self.page.clone()))
            } else {
                Ok(None)
            }
        }

        async fn find_by_id(&self, _id: Uuid) -> Result<Option<PageRecord>, RepoError> {
            Ok(None)
        }

        async fn count_pages(
            &self,
            _status: Option<PageStatus>,
            _filter: &PageQueryFilter,
        ) -> Result<u64, RepoError> {
            Ok(0)
        }

        async fn list_month_counts(
            &self,
            _status: Option<PageStatus>,
            _filter: &PageQueryFilter,
        ) -> Result<Vec<MonthCount>, RepoError> {
            Ok(Vec::new())
        }
    }

    fn sample_page(slug: &str) -> PageRecord {
        let now = OffsetDateTime::now_utc();
        PageRecord {
            id: Uuid::new_v4(),
            slug: slug.to_string(),
            title: "Sample".to_string(),
            body_markdown: "body".to_string(),
            rendered_html: "<p>Hello</p>".to_string(),
            status: PageStatus::Published,
            scheduled_at: None,
            published_at: Some(now),
            archived_at: None,
            created_at: now,
            updated_at: now,
        }
    }

    #[tokio::test]
    async fn page_view_uses_l0_cache() {
        let calls = Arc::new(AtomicUsize::new(0));
        let repo = Arc::new(StubPagesRepo {
            calls: calls.clone(),
            page: sample_page("about"),
        });

        let config = CacheConfig::default();
        let cache = Arc::new(L0Store::new(&config));
        let service = PageService::new(repo, Some(cache));

        let first = service.page_view("about").await.expect("first view");
        assert!(first.is_some());
        assert_eq!(calls.load(Ordering::SeqCst), 1);

        let second = service.page_view("about").await.expect("second view");
        assert!(second.is_some());
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }
}
