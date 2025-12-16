//! Syndication service for RSS and Atom feed generation.
//!
//! This service encapsulates the business logic for generating syndication feeds,
//! keeping the HTTP layer focused on request/response handling.

use std::sync::Arc;

use thiserror::Error;
use time::format_description::well_known::{Rfc2822, Rfc3339};

use crate::application::pagination::PageRequest;
use crate::application::repos::{PostListScope, PostQueryFilter, PostsRepo, RepoError, SettingsRepo};
use crate::domain::types::PostStatus;

/// Service for generating RSS and Atom feeds.
#[derive(Clone)]
pub struct SyndicationService {
    posts: Arc<dyn PostsRepo>,
    settings: Arc<dyn SettingsRepo>,
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
    pub fn new(posts: Arc<dyn PostsRepo>, settings: Arc<dyn SettingsRepo>) -> Self {
        Self { posts, settings }
    }

    /// Generate RSS 2.0 feed XML.
    ///
    /// Records cache dependencies: Feed, SiteSettings, PostsIndex.
    pub async fn rss_feed(&self) -> Result<String, SyndicationError> {
        // Record dependencies for L1 cache invalidation
        crate::cache::deps::record(crate::cache::EntityKey::Feed);
        crate::cache::deps::record(crate::cache::EntityKey::SiteSettings);
        crate::cache::deps::record(crate::cache::EntityKey::PostsIndex);

        let settings = self
            .settings
            .load_site_settings()
            .await
            .map_err(|e| SyndicationError::Settings(e.to_string()))?;

        let base = normalize_public_site_url(&settings.public_site_url);

        let page = self
            .posts
            .list_posts(
                PostListScope::Public,
                &PostQueryFilter::default(),
                PageRequest::new(100, None),
            )
            .await?;

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

        let settings = self
            .settings
            .load_site_settings()
            .await
            .map_err(|e| SyndicationError::Settings(e.to_string()))?;

        let base = normalize_public_site_url(&settings.public_site_url);

        let page = self
            .posts
            .list_posts(
                PostListScope::Public,
                &PostQueryFilter::default(),
                PageRequest::new(100, None),
            )
            .await?;

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
