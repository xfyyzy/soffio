//! Syndication service for RSS and Atom feed generation.
//!
//! This service encapsulates the business logic for generating syndication feeds,
//! keeping the HTTP layer focused on request/response handling.

use std::sync::Arc;

use thiserror::Error;
use time::format_description::well_known::{Rfc2822, Rfc3339};

use crate::application::pagination::PageRequest;
use crate::application::repos::{
    PostListScope, PostQueryFilter, PostsRepo, RepoError, SettingsRepo,
};
use crate::cache::{L0Store, hash_cursor_str, hash_post_list_key};
use crate::domain::types::PostStatus;

/// Service for generating RSS and Atom feeds.
#[derive(Clone)]
pub struct SyndicationService {
    posts: Arc<dyn PostsRepo>,
    settings: Arc<dyn SettingsRepo>,
    cache: Option<Arc<L0Store>>,
}

#[derive(Debug, Error)]
pub enum SyndicationError {
    #[error("failed to load settings: {0}")]
    Settings(String),
    #[error("failed to list posts: {0}")]
    Posts(String),
}

impl From<RepoError> for SyndicationError {
    fn from(err: RepoError) -> Self {
        SyndicationError::Posts(err.to_string())
    }
}

impl SyndicationService {
    pub fn new(
        posts: Arc<dyn PostsRepo>,
        settings: Arc<dyn SettingsRepo>,
        cache: Option<Arc<L0Store>>,
    ) -> Self {
        Self {
            posts,
            settings,
            cache,
        }
    }

    /// Generate RSS 2.0 feed XML.
    ///
    /// Records cache dependencies: Feed, SiteSettings, PostsIndex.
    pub async fn rss_feed(&self) -> Result<String, SyndicationError> {
        // Record dependencies for L1 cache invalidation
        crate::cache::deps::record(crate::cache::EntityKey::Feed);
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
                    .map_err(|e| SyndicationError::Settings(e.to_string()))?;
                cache.set_site_settings(settings.clone());
                settings
            }
        } else {
            self.settings
                .load_site_settings()
                .await
                .map_err(|e| SyndicationError::Settings(e.to_string()))?
        };

        let base = normalize_public_site_url(&settings.public_site_url);

        let filter = PostQueryFilter::default();
        let page_limit = 100u32;
        let filter_hash = hash_post_list_key(&filter, page_limit);
        let cursor_hash = hash_cursor_str(None);
        let page = if let Some(cache) = &self.cache {
            if let Some(cached) = cache.get_post_list(filter_hash, cursor_hash) {
                cached
            } else {
                let page = self
                    .posts
                    .list_posts(
                        PostListScope::Public,
                        &filter,
                        PageRequest::new(page_limit, None),
                    )
                    .await?;
                cache.set_post_list(filter_hash, cursor_hash, page.clone());
                page
            }
        } else {
            self.posts
                .list_posts(
                    PostListScope::Public,
                    &filter,
                    PageRequest::new(page_limit, None),
                )
                .await?
        };

        let mut items = String::new();
        for post in page
            .items
            .into_iter()
            .filter(|p| p.status == PostStatus::Published)
        {
            let published = post.published_at.unwrap_or(post.updated_at);
            let pub_date = published
                .format(&Rfc2822)
                .unwrap_or_else(|_| published.to_string());
            let link = format!("{base}posts/{}", post.slug);
            items.push_str(&format!(
                "    <item>\n      <title>{}</title>\n      <link>{}</link>\n      <guid>{}</guid>\n      <pubDate>{}</pubDate>\n      <description><![CDATA[{}]]></description>\n    </item>\n",
                xml_escape(&post.title),
                link,
                link,
                pub_date,
                xml_escape(&post.excerpt),
            ));
        }

        let channel = format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<rss version=\"2.0\">\n  <channel>\n    <title>{}</title>\n    <link>{}</link>\n    <description>{}</description>\n{}  </channel>\n</rss>\n",
            xml_escape(&settings.meta_title),
            base,
            xml_escape(&settings.meta_description),
            items
        );

        Ok(channel)
    }

    /// Generate Atom 1.0 feed XML.
    ///
    /// Records cache dependencies: Feed, SiteSettings, PostsIndex.
    pub async fn atom_feed(&self) -> Result<String, SyndicationError> {
        // Record dependencies for L1 cache invalidation
        crate::cache::deps::record(crate::cache::EntityKey::Feed);
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
                    .map_err(|e| SyndicationError::Settings(e.to_string()))?;
                cache.set_site_settings(settings.clone());
                settings
            }
        } else {
            self.settings
                .load_site_settings()
                .await
                .map_err(|e| SyndicationError::Settings(e.to_string()))?
        };

        let base = normalize_public_site_url(&settings.public_site_url);

        let filter = PostQueryFilter::default();
        let page_limit = 100u32;
        let filter_hash = hash_post_list_key(&filter, page_limit);
        let cursor_hash = hash_cursor_str(None);
        let page = if let Some(cache) = &self.cache {
            if let Some(cached) = cache.get_post_list(filter_hash, cursor_hash) {
                cached
            } else {
                let page = self
                    .posts
                    .list_posts(
                        PostListScope::Public,
                        &filter,
                        PageRequest::new(page_limit, None),
                    )
                    .await?;
                cache.set_post_list(filter_hash, cursor_hash, page.clone());
                page
            }
        } else {
            self.posts
                .list_posts(
                    PostListScope::Public,
                    &filter,
                    PageRequest::new(page_limit, None),
                )
                .await?
        };

        let updated = settings
            .updated_at
            .format(&Rfc3339)
            .unwrap_or_else(|_| settings.updated_at.to_string());

        let mut entries = String::new();
        for post in page
            .items
            .into_iter()
            .filter(|p| p.status == PostStatus::Published)
        {
            let published = post.published_at.unwrap_or(post.updated_at);
            let published_str = published
                .format(&Rfc3339)
                .unwrap_or_else(|_| published.to_string());
            let link = format!("{base}posts/{}", post.slug);
            entries.push_str(&format!(
                "  <entry>\n    <title>{}</title>\n    <link href=\"{}\"/>\n    <id>{}</id>\n    <updated>{}</updated>\n    <summary><![CDATA[{}]]></summary>\n  </entry>\n",
                xml_escape(&post.title),
                link,
                link,
                published_str,
                xml_escape(&post.excerpt),
            ));
        }

        let feed = format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<feed xmlns=\"http://www.w3.org/2005/Atom\">\n  <title>{}</title>\n  <id>{}</id>\n  <updated>{}</updated>\n  <link href=\"{}atom.xml\" rel=\"self\"/>\n{}\n</feed>\n",
            xml_escape(&settings.meta_title),
            base,
            updated,
            base,
            entries
        );

        Ok(feed)
    }
}

fn normalize_public_site_url(url: &str) -> String {
    let trimmed = url.trim_end_matches('/');
    format!("{trimmed}/")
}

fn xml_escape(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}
