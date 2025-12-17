//! Sitemap service for sitemap.xml and robots.txt generation.
//!
//! This service encapsulates the business logic for generating sitemap and robots.txt,
//! keeping the HTTP layer focused on request/response handling.

use std::sync::Arc;

use thiserror::Error;
use time::format_description::well_known::Rfc3339;

use crate::application::pagination::{PageCursor, PageRequest, PostCursor};
use crate::application::repos::{
    PageQueryFilter, PagesRepo, PostListScope, PostQueryFilter, PostsRepo, RepoError, SettingsRepo,
};
use crate::cache::L0Store;
use crate::domain::types::{PageStatus, PostStatus};

/// Service for generating sitemap.xml and robots.txt.
#[derive(Clone)]
pub struct SitemapService {
    posts: Arc<dyn PostsRepo>,
    pages: Arc<dyn PagesRepo>,
    settings: Arc<dyn SettingsRepo>,
    cache: Option<Arc<L0Store>>,
}

#[derive(Debug, Error)]
pub enum SitemapError {
    #[error("failed to load settings: {0}")]
    Settings(String),
    #[error("failed to list posts: {0}")]
    Posts(String),
    #[error("failed to list pages: {0}")]
    Pages(String),
    #[error("failed to decode cursor: {0}")]
    Cursor(String),
}

impl From<RepoError> for SitemapError {
    fn from(err: RepoError) -> Self {
        SitemapError::Posts(err.to_string())
    }
}

impl SitemapService {
    pub fn new(
        posts: Arc<dyn PostsRepo>,
        pages: Arc<dyn PagesRepo>,
        settings: Arc<dyn SettingsRepo>,
        cache: Option<Arc<L0Store>>,
    ) -> Self {
        Self {
            posts,
            pages,
            settings,
            cache,
        }
    }

    /// Generate sitemap.xml content.
    ///
    /// Records cache dependencies: Sitemap, SiteSettings, PostsIndex.
    pub async fn sitemap_xml(&self) -> Result<String, SitemapError> {
        // Record dependencies for L1 cache invalidation
        crate::cache::deps::record(crate::cache::EntityKey::Sitemap);
        crate::cache::deps::record(crate::cache::EntityKey::SiteSettings);
        crate::cache::deps::record(crate::cache::EntityKey::PostsIndex);

        let settings = if let Some(cache) = &self.cache {
            if let Some(cached) = cache.get_site_settings() {
                cached
            } else {
                let settings = self
                    .settings
                    .load_site_settings()
                    .await
                    .map_err(|e| SitemapError::Settings(e.to_string()))?;
                cache.set_site_settings(settings.clone());
                settings
            }
        } else {
            self.settings
                .load_site_settings()
                .await
                .map_err(|e| SitemapError::Settings(e.to_string()))?
        };

        let base = normalize_public_site_url(&settings.public_site_url);
        let mut entries = Vec::new();

        // Homepage entry
        entries.push(sitemap_entry(&base, "/", Some(settings.updated_at)));

        // Posts
        let mut post_cursor: Option<PostCursor> = None;
        loop {
            let page = self
                .posts
                .list_posts(
                    PostListScope::Public,
                    &PostQueryFilter::default(),
                    PageRequest::new(200, post_cursor),
                )
                .await?;

            for post in page.items.into_iter() {
                if post.status != PostStatus::Published {
                    continue;
                }
                let lastmod = post.published_at.unwrap_or(post.updated_at);
                entries.push(sitemap_entry(
                    &base,
                    &format!("/posts/{}", post.slug),
                    Some(lastmod),
                ));
            }

            post_cursor = match page.next_cursor {
                Some(next) => Some(
                    PostCursor::decode(&next).map_err(|e| SitemapError::Cursor(e.to_string()))?,
                ),
                None => break,
            };
        }

        // Pages
        let mut page_cursor: Option<PageCursor> = None;
        loop {
            let page = self
                .pages
                .list_pages(
                    Some(PageStatus::Published),
                    200,
                    page_cursor,
                    &PageQueryFilter::default(),
                )
                .await
                .map_err(|e| SitemapError::Pages(e.to_string()))?;

            for record in page.items.into_iter() {
                if record.published_at.is_none() {
                    continue;
                }
                let lastmod = record.published_at.unwrap_or(record.updated_at);
                entries.push(sitemap_entry(
                    &base,
                    &format!("/{}", record.slug),
                    Some(lastmod),
                ));
            }

            page_cursor = match page.next_cursor {
                Some(next) => Some(
                    PageCursor::decode(&next).map_err(|e| SitemapError::Cursor(e.to_string()))?,
                ),
                None => break,
            };
        }

        let mut xml = String::from(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">\n",
        );
        for entry in entries {
            xml.push_str(&entry);
        }
        xml.push_str("</urlset>\n");
        Ok(xml)
    }

    /// Generate robots.txt content.
    ///
    /// Records cache dependencies: SiteSettings.
    pub async fn robots_txt(&self) -> Result<String, SitemapError> {
        // Record dependencies for L1 cache invalidation
        crate::cache::deps::record(crate::cache::EntityKey::SiteSettings);

        let settings = if let Some(cache) = &self.cache {
            if let Some(cached) = cache.get_site_settings() {
                cached
            } else {
                let settings = self
                    .settings
                    .load_site_settings()
                    .await
                    .map_err(|e| SitemapError::Settings(e.to_string()))?;
                cache.set_site_settings(settings.clone());
                settings
            }
        } else {
            self.settings
                .load_site_settings()
                .await
                .map_err(|e| SitemapError::Settings(e.to_string()))?
        };

        let base = normalize_public_site_url(&settings.public_site_url);
        let sitemap_url = format!("{base}sitemap.xml");
        let body = format!("User-agent: *\nAllow: /\nSitemap: {sitemap_url}\n");
        Ok(body)
    }
}

fn normalize_public_site_url(url: &str) -> String {
    let trimmed = url.trim_end_matches('/');
    format!("{trimmed}/")
}

fn sitemap_entry(base: &str, path: &str, lastmod: Option<time::OffsetDateTime>) -> String {
    let loc = canonical_url(base, path);
    let lastmod_str = lastmod
        .and_then(|dt| dt.format(&Rfc3339).ok())
        .unwrap_or_default();
    if lastmod_str.is_empty() {
        format!("  <url><loc>{loc}</loc></url>\n")
    } else {
        format!("  <url><loc>{loc}</loc><lastmod>{lastmod_str}</lastmod></url>\n")
    }
}

fn canonical_url(base: &str, path: &str) -> String {
    let base = base.trim_end_matches('/');
    if path == "/" {
        base.to_string()
    } else {
        format!("{base}{path}")
    }
}
