use std::{collections::HashSet, sync::Arc};

use axum::http::StatusCode;
use thiserror::Error;
use tracing::{info, warn};

use crate::{
    application::{feed::FeedFilter, repos::TagsRepo},
    infra::{
        cache::{CacheStoreError, should_store_response},
        db::PostgresRepositories,
        http::{
            HttpState,
            public::{canonical_url, page_meta, post_meta},
        },
    },
    presentation::views::{
        IndexTemplate, LayoutChrome, LayoutContext, PageTemplate, PostTemplate,
        render_template_response,
    },
};

#[derive(Debug, Error)]
pub enum CacheWarmError {
    #[error("failed to load site chrome: {0}")]
    Chrome(String),
    #[error("failed to load feed content: {0}")]
    Feed(String),
    #[error("failed to load post `{slug}`: {detail}")]
    PostDetail { slug: String, detail: String },
    #[error("failed to load page `{slug}`: {detail}")]
    PageView { slug: String, detail: String },
    #[error("database query failed: {0}")]
    Database(String),
    #[error("failed to store cached response for `{path}`: {source}")]
    Cache {
        path: String,
        #[source]
        source: CacheStoreError,
    },
}

pub struct CacheWarmer {
    state: HttpState,
}

impl CacheWarmer {
    pub fn new(state: HttpState) -> Self {
        Self { state }
    }

    pub async fn warm_initial(&self) -> Result<(), CacheWarmError> {
        info!(target = "soffio::cache_warmer", "warming public cache");

        let chrome = self
            .state
            .chrome
            .load()
            .await
            .map_err(|err| CacheWarmError::Chrome(format!("{err:?}")))?;

        let mut warmed_paths = HashSet::new();
        let mut warmed_posts = HashSet::new();

        self.warm_home(&chrome, &mut warmed_paths, &mut warmed_posts)
            .await?;
        self.warm_pinned_tags(&chrome, &mut warmed_paths, &mut warmed_posts)
            .await?;
        self.warm_published_pages(&chrome, &mut warmed_paths)
            .await?;

        Ok(())
    }

    async fn warm_home(
        &self,
        chrome: &LayoutChrome,
        warmed_paths: &mut HashSet<String>,
        warmed_posts: &mut HashSet<String>,
    ) -> Result<(), CacheWarmError> {
        let content = self
            .state
            .feed
            .page_context(FeedFilter::All, None)
            .await
            .map_err(|err| CacheWarmError::Feed(err.to_string()))?;

        let post_slugs: Vec<String> = content.posts.iter().map(|post| post.slug.clone()).collect();

        let canonical = canonical_url(&chrome.meta.canonical, "/");
        let response = render_template_response(
            IndexTemplate {
                view: LayoutContext::new(chrome.clone().with_canonical(canonical), content),
            },
            StatusCode::OK,
        );
        self.store_if_needed("/", response, warmed_paths).await?;

        for slug in post_slugs {
            self.warm_post_detail(&slug, chrome, warmed_paths, warmed_posts)
                .await?;
        }

        Ok(())
    }

    async fn warm_pinned_tags(
        &self,
        chrome: &LayoutChrome,
        warmed_paths: &mut HashSet<String>,
        warmed_posts: &mut HashSet<String>,
    ) -> Result<(), CacheWarmError> {
        let pinned_slugs = self.load_pinned_tag_slugs().await?;
        if pinned_slugs.is_empty() {
            return Ok(());
        }

        for slug in pinned_slugs {
            let content = self
                .state
                .feed
                .page_context(FeedFilter::Tag(slug.clone()), None)
                .await
                .map_err(|err| CacheWarmError::Feed(err.to_string()))?;

            let post_slugs: Vec<String> =
                content.posts.iter().map(|post| post.slug.clone()).collect();

            let path = format!("/tags/{slug}");
            let canonical = canonical_url(&chrome.meta.canonical, &path);
            let response = render_template_response(
                IndexTemplate {
                    view: LayoutContext::new(chrome.clone().with_canonical(canonical), content),
                },
                StatusCode::OK,
            );
            self.store_if_needed(&path, response, warmed_paths).await?;

            for post_slug in post_slugs {
                self.warm_post_detail(&post_slug, chrome, warmed_paths, warmed_posts)
                    .await?;
            }
        }

        Ok(())
    }

    async fn warm_published_pages(
        &self,
        chrome: &LayoutChrome,
        warmed_paths: &mut HashSet<String>,
    ) -> Result<(), CacheWarmError> {
        let slugs = load_published_page_slugs(self.state.db.clone())
            .await
            .map_err(CacheWarmError::Database)?;

        for slug in slugs {
            if slug.is_empty() {
                continue;
            }

            let view = match self.state.pages.page_view(&slug).await {
                Ok(Some(view)) => view,
                Ok(None) => {
                    warn!(
                        target = "soffio::cache_warmer",
                        slug = %slug,
                        "published page missing rendered view"
                    );
                    continue;
                }
                Err(err) => {
                    return Err(CacheWarmError::PageView {
                        slug,
                        detail: format!("{err:?}"),
                    });
                }
            };

            let canonical = canonical_url(&chrome.meta.canonical, &format!("/{slug}"));
            let meta = page_meta(chrome, &view, canonical);
            let response = render_template_response(
                PageTemplate {
                    view: LayoutContext::new(chrome.clone().with_meta(meta), view),
                },
                StatusCode::OK,
            );

            let path = format!("/{slug}");
            self.store_if_needed(&path, response, warmed_paths).await?;
        }

        Ok(())
    }

    async fn warm_post_detail(
        &self,
        slug: &str,
        chrome: &LayoutChrome,
        warmed_paths: &mut HashSet<String>,
        warmed_posts: &mut HashSet<String>,
    ) -> Result<(), CacheWarmError> {
        if !warmed_posts.insert(slug.to_string()) {
            return Ok(());
        }

        let detail = match self.state.feed.post_detail(slug).await {
            Ok(Some(detail)) => detail,
            Ok(None) => {
                warn!(
                    target = "soffio::cache_warmer",
                    slug = %slug,
                    "skipping post cache warm because detail not available"
                );
                return Ok(());
            }
            Err(err) => {
                return Err(CacheWarmError::PostDetail {
                    slug: slug.to_string(),
                    detail: err.to_string(),
                });
            }
        };

        let path = format!("/posts/{slug}");
        let canonical = canonical_url(&chrome.meta.canonical, &path);
        let response = render_template_response(
            PostTemplate {
                view: LayoutContext::new(
                    chrome
                        .clone()
                        .with_meta(post_meta(chrome, &detail, canonical)),
                    detail,
                ),
            },
            StatusCode::OK,
        );

        self.store_if_needed(&path, response, warmed_paths).await
    }

    async fn store_if_needed(
        &self,
        path: &str,
        response: axum::response::Response,
        warmed_paths: &mut HashSet<String>,
    ) -> Result<(), CacheWarmError> {
        if warmed_paths.insert(path.to_string()) {
            self.store(path, response).await
        } else {
            Ok(())
        }
    }

    async fn store(
        &self,
        path: &str,
        response: axum::response::Response,
    ) -> Result<(), CacheWarmError> {
        if !should_store_response(&response) {
            return Ok(());
        }

        match self.state.cache.store_response(path, response).await {
            Ok(_) => {
                info!(
                    target = "soffio::cache_warmer",
                    path = %path,
                    "cache entry warmed"
                );
                Ok(())
            }
            Err((_, error)) => Err(CacheWarmError::Cache {
                path: path.to_string(),
                source: error,
            }),
        }
    }

    async fn load_pinned_tag_slugs(&self) -> Result<Vec<String>, CacheWarmError> {
        let tags = self
            .state
            .db
            .list_all()
            .await
            .map_err(|err| CacheWarmError::Database(err.to_string()))?;

        Ok(tags
            .into_iter()
            .filter(|tag| tag.pinned)
            .map(|tag| tag.slug)
            .collect())
    }
}

async fn load_published_page_slugs(
    repositories: Arc<PostgresRepositories>,
) -> Result<Vec<String>, String> {
    sqlx::query!(
        r#"
        SELECT p.slug
        FROM navigation_items ni
        INNER JOIN pages p ON p.id = ni.destination_page_id
        WHERE ni.destination_type = 'internal'::navigation_destination_type
          AND ni.visible = TRUE
          AND p.status = 'published'::page_status
          AND p.published_at IS NOT NULL
        ORDER BY ni.sort_order ASC, ni.id ASC
        "#
    )
    .fetch_all(repositories.pool())
    .await
    .map(|rows| rows.into_iter().map(|row| row.slug).collect())
    .map_err(|err| err.to_string())
}

#[cfg(test)]
mod tests {}
