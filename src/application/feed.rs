use std::sync::Arc;

use askama::Template;
use axum::response::Response;
use datastar::prelude::ElementPatchMode;
use serde_json::json;
use thiserror::Error;

use crate::application::error::HttpError;
use crate::application::pagination::{PageRequest, PostCursor};
use crate::application::repos::{
    PostListScope, PostQueryFilter, PostsRepo, RepoError, SectionsRepo, SettingsRepo, TagWithCount,
    TagsRepo,
};
use crate::application::stream::StreamBuilder;
use crate::cache::{L0Store, hash_cursor_str, hash_post_list_key};
use crate::domain::entities::{PostRecord, SiteSettingsRecord, TagRecord};
use crate::domain::posts;
use crate::domain::sections::{PostSectionNode, SectionTreeError, build_section_tree};
use crate::domain::types::PostStatus;
use crate::presentation::views::{
    self, FeedLoaderContext, FeedLoaderTemplate, PageContext, PostCard, PostCardsAppendTemplate,
    PostDetailContext, PostSectionEvent, PostTocEvent, PostTocView, TemplateRenderError,
    build_tag_badges,
};
use crate::util::timezone;
use uuid::Uuid;
const DEFAULT_PAGE_SIZE: usize = 6;

#[path = "feed/presentation.rs"]
mod presentation;
#[path = "feed/sections.rs"]
mod sections;
#[path = "feed/summaries.rs"]
mod summaries;
#[derive(Clone)]
pub enum FeedFilter {
    All,
    Tag(String),
    Month(String),
}

impl FeedFilter {
    pub fn tag(&self) -> Option<&str> {
        match self {
            FeedFilter::Tag(value) => Some(value.as_str()),
            _ => None,
        }
    }

    pub fn month(&self) -> Option<&str> {
        match self {
            FeedFilter::Month(value) => Some(value.as_str()),
            _ => None,
        }
    }

    pub fn load_more_query(&self) -> String {
        match self {
            FeedFilter::All => String::new(),
            FeedFilter::Tag(value) => format!("&tag={value}"),
            FeedFilter::Month(value) => format!("&month={value}"),
        }
    }

    pub fn base_path(&self) -> String {
        match self {
            FeedFilter::All => "/".to_string(),
            FeedFilter::Tag(value) => format!("/tags/{value}"),
            FeedFilter::Month(value) => format!("/months/{value}"),
        }
    }

    fn to_query_filter(&self) -> PostQueryFilter {
        let mut filter = PostQueryFilter::default();
        match self {
            FeedFilter::All => {}
            FeedFilter::Tag(tag) => filter.tag = Some(tag.clone()),
            FeedFilter::Month(month) => filter.month = Some(month.clone()),
        }
        filter
    }
}

#[derive(Clone)]
pub struct AppendPayload {
    pub offset: usize,
    pub cards: Vec<PostCard>,
    pub next_cursor: Option<String>,
    pub total_visible: usize,
}

#[derive(Clone)]
pub struct FeedService {
    posts: Arc<dyn PostsRepo>,
    sections: Arc<dyn SectionsRepo>,
    tags: Arc<dyn TagsRepo>,
    settings: Arc<dyn SettingsRepo>,
    cache: Option<Arc<L0Store>>,
}

#[derive(Debug, Error)]
pub enum FeedError {
    #[error("invalid cursor: {0}")]
    InvalidCursor(String),
    #[error("unknown tag")]
    UnknownTag,
    #[error("unknown month")]
    UnknownMonth,
    #[error("invalid section hierarchy: {0}")]
    SectionTree(#[from] SectionTreeError),
    #[error(transparent)]
    Repo(#[from] RepoError),
}

impl FeedService {
    pub fn new(
        posts: Arc<dyn PostsRepo>,
        sections: Arc<dyn SectionsRepo>,
        tags: Arc<dyn TagsRepo>,
        settings: Arc<dyn SettingsRepo>,
        cache: Option<Arc<L0Store>>,
    ) -> Self {
        Self {
            posts,
            sections,
            tags,
            settings,
            cache,
        }
    }

    fn decode_cursor(&self, cursor: Option<&str>) -> Result<Option<PostCursor>, FeedError> {
        cursor
            .map(PostCursor::decode)
            .transpose()
            .map_err(|err| FeedError::InvalidCursor(err.to_string()))
    }

    pub async fn page_context(
        &self,
        filter: FeedFilter,
        cursor: Option<&str>,
    ) -> Result<PageContext, FeedError> {
        // Record derived collection dependencies for cache invalidation
        crate::cache::deps::record(crate::cache::EntityKey::PostsIndex);
        crate::cache::deps::record(crate::cache::EntityKey::PostAggTags);
        crate::cache::deps::record(crate::cache::EntityKey::PostAggMonths);

        let decoded_cursor = self.decode_cursor(cursor)?;
        let query_filter = filter.to_query_filter();
        let settings = self.load_site_settings().await?;
        let page_limit = homepage_page_limit(&settings);

        let filter_hash = hash_post_list_key(&query_filter, page_limit);
        let cursor_hash = hash_cursor_str(cursor);

        let page = if let Some(cache) = &self.cache {
            if let Some(cached) = cache.get_post_list(filter_hash, cursor_hash) {
                cached
            } else {
                let page = self
                    .posts
                    .list_posts(
                        PostListScope::Public,
                        &query_filter,
                        PageRequest::new(page_limit, decoded_cursor),
                    )
                    .await?;
                cache.set_post_list(filter_hash, cursor_hash, page.clone());
                page
            }
        } else {
            self.posts
                .list_posts(
                    PostListScope::Public,
                    &query_filter,
                    PageRequest::new(page_limit, decoded_cursor),
                )
                .await?
        };

        let total_filtered = self
            .posts
            .count_posts(PostListScope::Public, &query_filter)
            .await?;

        let total_all = self
            .posts
            .count_posts(PostListScope::Public, &PostQueryFilter::default())
            .await?;

        let tag_counts = if let Some(cache) = &self.cache {
            if let Some(cached) = cache.get_tag_counts() {
                cached
            } else {
                let tags = self.tags.list_with_counts().await?;
                cache.set_tag_counts(tags.clone());
                tags
            }
        } else {
            self.tags.list_with_counts().await?
        };
        let month_counts = if let Some(cache) = &self.cache {
            if let Some(cached) = cache.get_month_counts() {
                cached
            } else {
                let months = self
                    .posts
                    .list_month_counts(PostListScope::Public, &PostQueryFilter::default())
                    .await?;
                cache.set_month_counts(months.clone());
                months
            }
        } else {
            self.posts
                .list_month_counts(PostListScope::Public, &PostQueryFilter::default())
                .await?
        };

        let tag_summaries = if settings.show_tag_aggregations {
            build_tag_summaries(&tag_counts, filter.tag(), total_all, &settings)
        } else {
            Vec::new()
        };

        let month_summaries = if settings.show_month_aggregations {
            build_month_summaries(
                &month_counts,
                filter.month(),
                total_all,
                settings.month_filter_limit,
            )
        } else {
            Vec::new()
        };

        let mut cards = Vec::with_capacity(page.items.len());
        for record in &page.items {
            let tags = self.tags.list_for_post(record.id).await?;
            cards.push(record_to_card(record, &tags, settings.timezone));
        }

        let posts_ld_json = build_posts_ld_json(
            &cards,
            &filter,
            &settings.public_site_url,
            &settings.meta_title,
        );

        let post_count = cards.len();
        Ok(PageContext {
            posts: cards,
            post_count,
            total_count: usize::try_from(total_filtered).unwrap_or(usize::MAX),
            has_results: post_count > 0,
            tags: tag_summaries,
            months: month_summaries,
            show_tag_filters: settings.show_tag_aggregations,
            show_month_filters: settings.show_month_aggregations,
            next_cursor: page.next_cursor,
            load_more_query: filter.load_more_query(),
            posts_ld_json,
        })
    }

    pub async fn append_payload(
        &self,
        filter: FeedFilter,
        cursor: Option<&str>,
    ) -> Result<AppendPayload, FeedError> {
        let decoded_cursor = self.decode_cursor(cursor)?;
        let query_filter = filter.to_query_filter();
        let settings = self.load_site_settings().await?;
        let page_limit = homepage_page_limit(&settings);
        let filter_hash = hash_post_list_key(&query_filter, page_limit);
        let cursor_hash = hash_cursor_str(cursor);

        let page = if let Some(cache) = &self.cache {
            if let Some(cached) = cache.get_post_list(filter_hash, cursor_hash) {
                cached
            } else {
                let page = self
                    .posts
                    .list_posts(
                        PostListScope::Public,
                        &query_filter,
                        PageRequest::new(page_limit, decoded_cursor),
                    )
                    .await?;
                cache.set_post_list(filter_hash, cursor_hash, page.clone());
                page
            }
        } else {
            self.posts
                .list_posts(
                    PostListScope::Public,
                    &query_filter,
                    PageRequest::new(page_limit, decoded_cursor),
                )
                .await?
        };

        let mut cards = Vec::with_capacity(page.items.len());
        for record in &page.items {
            let tags = self.tags.list_for_post(record.id).await?;
            cards.push(record_to_card(record, &tags, settings.timezone));
        }

        let offset = if let Some(cursor) = decoded_cursor {
            self.posts
                .count_posts_before(PostListScope::Public, &query_filter, &cursor)
                .await?
        } else {
            0
        };

        let offset_usize = usize::try_from(offset).unwrap_or(usize::MAX);
        let total_visible = offset_usize.saturating_add(cards.len());

        Ok(AppendPayload {
            offset: offset_usize,
            cards,
            next_cursor: page.next_cursor,
            total_visible,
        })
    }

    pub async fn post_detail(&self, slug: &str) -> Result<Option<PostDetailContext>, FeedError> {
        // Record post slug dependency for cache invalidation
        crate::cache::deps::record(crate::cache::EntityKey::PostSlug(slug.to_string()));

        let post = if let Some(cache) = &self.cache {
            if let Some(cached) = cache.get_post_by_slug(slug) {
                Some(cached)
            } else {
                let fetched = self.posts.find_by_slug(slug).await?;
                if let Some(post) = fetched.clone() {
                    cache.set_post(post);
                }
                fetched
            }
        } else {
            self.posts.find_by_slug(slug).await?
        };

        let Some(post) = post else {
            return Ok(None);
        };

        if post.status != PostStatus::Published || post.published_at.is_none() {
            return Ok(None);
        }

        self.build_post_context(post).await.map(Some)
    }

    pub async fn post_preview(&self, id: Uuid) -> Result<Option<PostDetailContext>, FeedError> {
        let Some(post) = self.posts.find_by_id(id).await? else {
            return Ok(None);
        };

        self.build_post_context(post).await.map(Some)
    }

    async fn build_post_context(&self, post: PostRecord) -> Result<PostDetailContext, FeedError> {
        let sections = self.sections.list_sections(post.id).await?;
        let section_nodes = build_section_tree(sections)?;
        let tags = self.tags.list_for_post(post.id).await?;
        let settings = self.load_site_settings().await?;

        let has_code_blocks = PostSectionNode::any_contains_code(&section_nodes);
        let has_math_blocks = PostSectionNode::any_contains_math(&section_nodes);
        let has_mermaid_diagrams = PostSectionNode::any_contains_mermaid(&section_nodes);
        let sections = build_post_section_events(&section_nodes);
        let toc = if settings.global_toc_enabled {
            build_post_toc_view(&section_nodes)
        } else {
            None
        };

        let published_at = post.published_at.unwrap_or(post.created_at);
        let localized = timezone::localized_datetime(published_at, settings.timezone);
        let date = timezone::localized_date(published_at, settings.timezone);

        Ok(PostDetailContext {
            slug: post.slug,
            title: post.title,
            published: posts::format_human_date(date),
            iso_date: localized.to_rfc3339(),
            tags: build_tag_badges(
                tags.iter()
                    .map(|tag| (tag.slug.as_str(), tag.name.as_str())),
            ),
            excerpt: post.excerpt,
            summary_html: post.summary_html,
            sections,
            has_code_blocks,
            has_math_blocks,
            has_mermaid_diagrams,
            toc,
            is_pinned: post.pinned,
        })
    }

    pub async fn is_known_tag(&self, tag: &str) -> Result<bool, FeedError> {
        crate::cache::deps::record(crate::cache::EntityKey::PostAggTags);

        let tags = if let Some(cache) = &self.cache {
            if let Some(cached) = cache.get_tag_counts() {
                cached
            } else {
                let tags = self.tags.list_with_counts().await?;
                cache.set_tag_counts(tags.clone());
                tags
            }
        } else {
            self.tags.list_with_counts().await?
        };
        Ok(tags.iter().any(|record| record.slug == tag))
    }

    pub async fn is_known_month(&self, month: &str) -> Result<bool, FeedError> {
        crate::cache::deps::record(crate::cache::EntityKey::PostAggMonths);

        let months = if let Some(cache) = &self.cache {
            if let Some(cached) = cache.get_month_counts() {
                cached
            } else {
                let months = self
                    .posts
                    .list_month_counts(PostListScope::Public, &PostQueryFilter::default())
                    .await?;
                cache.set_month_counts(months.clone());
                months
            }
        } else {
            self.posts
                .list_month_counts(PostListScope::Public, &PostQueryFilter::default())
                .await?
        };
        Ok(months.iter().any(|entry| entry.key == month))
    }

    async fn load_site_settings(&self) -> Result<SiteSettingsRecord, FeedError> {
        // Record site settings dependency for cache invalidation
        crate::cache::deps::record(crate::cache::EntityKey::SiteSettings);

        if let Some(settings) = self
            .cache
            .as_ref()
            .and_then(|cache| cache.get_site_settings())
        {
            return Ok(settings);
        }

        let settings = self
            .settings
            .load_site_settings()
            .await
            .map_err(FeedError::from)?;

        if let Some(cache) = &self.cache {
            cache.set_site_settings(settings.clone());
        }

        Ok(settings)
    }
}

fn record_to_card(record: &PostRecord, tags: &[TagRecord], timezone: chrono_tz::Tz) -> PostCard {
    presentation::record_to_card(record, tags, timezone)
}

fn build_posts_ld_json(
    cards: &[PostCard],
    filter: &FeedFilter,
    public_site_url: &str,
    blog_name: &str,
) -> Option<String> {
    presentation::build_posts_ld_json(cards, filter, public_site_url, blog_name)
}

fn build_post_section_events(nodes: &[PostSectionNode]) -> Vec<PostSectionEvent> {
    sections::build_post_section_events(nodes)
}

fn build_post_toc_view(nodes: &[PostSectionNode]) -> Option<PostTocView> {
    sections::build_post_toc_view(nodes)
}

pub fn build_datastar_append_response(
    payload: AppendPayload,
    load_more_query: String,
) -> Result<Response, HttpError> {
    presentation::build_datastar_append_response(payload, load_more_query)
}

fn homepage_page_limit(settings: &SiteSettingsRecord) -> u32 {
    presentation::homepage_page_limit(settings)
}

pub(crate) fn order_tags_with_pins(counts: &[TagWithCount]) -> Vec<&TagWithCount> {
    summaries::order_tags_with_pins(counts)
}

fn build_tag_summaries(
    counts: &[TagWithCount],
    active_tag: Option<&str>,
    total_posts: u64,
    settings: &SiteSettingsRecord,
) -> Vec<views::TagSummary> {
    summaries::build_tag_summaries(counts, active_tag, total_posts, settings)
}

fn build_month_summaries(
    counts: &[posts::MonthCount],
    active: Option<&str>,
    total_posts: u64,
    limit: i32,
) -> Vec<views::MonthSummary> {
    summaries::build_month_summaries(counts, active, total_posts, limit)
}
