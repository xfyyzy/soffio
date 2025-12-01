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
    ) -> Self {
        Self {
            posts,
            sections,
            tags,
            settings,
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
        let decoded_cursor = self.decode_cursor(cursor)?;
        let query_filter = filter.to_query_filter();
        let settings = self.load_site_settings().await?;
        let page_limit = homepage_page_limit(&settings);

        let page = self
            .posts
            .list_posts(
                PostListScope::Public,
                &query_filter,
                PageRequest::new(page_limit, decoded_cursor),
            )
            .await?;

        let total_filtered = self
            .posts
            .count_posts(PostListScope::Public, &query_filter)
            .await?;

        let total_all = self
            .posts
            .count_posts(PostListScope::Public, &PostQueryFilter::default())
            .await?;

        let tag_counts = self.tags.list_with_counts().await?;
        let month_counts = self
            .posts
            .list_month_counts(PostListScope::Public, &PostQueryFilter::default())
            .await?;

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

        let page = self
            .posts
            .list_posts(
                PostListScope::Public,
                &query_filter,
                PageRequest::new(page_limit, decoded_cursor),
            )
            .await?;

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
        let Some(post) = self.posts.find_by_slug(slug).await? else {
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
        let tags = self.tags.list_all().await?;
        Ok(tags.iter().any(|record| record.slug == tag))
    }

    pub async fn is_known_month(&self, month: &str) -> Result<bool, FeedError> {
        let months = self
            .posts
            .list_month_counts(PostListScope::Public, &PostQueryFilter::default())
            .await?;
        Ok(months.iter().any(|entry| entry.key == month))
    }

    async fn load_site_settings(&self) -> Result<SiteSettingsRecord, FeedError> {
        self.settings
            .load_site_settings()
            .await
            .map_err(FeedError::from)
    }
}

fn record_to_card(record: &PostRecord, tags: &[TagRecord], timezone: chrono_tz::Tz) -> PostCard {
    let published_at = record.published_at.unwrap_or(record.created_at);
    let localized = timezone::localized_datetime(published_at, timezone);
    let date = timezone::localized_date(published_at, timezone);

    PostCard {
        slug: record.slug.clone(),
        title: record.title.clone(),
        excerpt: record.excerpt.clone(),
        iso_date: localized.to_rfc3339(),
        published: posts::format_human_date(date),
        badges: build_tag_badges(
            tags.iter()
                .map(|tag| (tag.slug.as_str(), tag.name.as_str())),
        ),
        is_pinned: record.pinned,
    }
}

fn build_posts_ld_json(
    cards: &[PostCard],
    filter: &FeedFilter,
    public_site_url: &str,
    blog_name: &str,
) -> Option<String> {
    if cards.is_empty() {
        return None;
    }

    let site_url = normalize_public_site_url(public_site_url);
    let blog_url = format!("{site_url}{}", filter.base_path());

    let blog_posts = cards
        .iter()
        .map(|card| {
            json!({
                "@type": "BlogPosting",
                "headline": card.title,
                "description": card.excerpt,
                "datePublished": card.iso_date,
                "url": format!("{site_url}posts/{}", card.slug),
            })
        })
        .collect::<Vec<_>>();

    serde_json::to_string(&json!({
        "@context": "https://schema.org",
        "@type": "Blog",
        "name": blog_name,
        "url": blog_url,
        "blogPost": blog_posts,
    }))
    .ok()
}

fn normalize_public_site_url(url: &str) -> String {
    let trimmed = url.trim_end_matches('/');
    format!("{trimmed}/")
}

fn build_post_section_events(nodes: &[PostSectionNode]) -> Vec<PostSectionEvent> {
    let mut events = Vec::new();
    for node in nodes {
        append_section_events(node, &mut events);
    }
    events
}

fn append_section_events(node: &PostSectionNode, events: &mut Vec<PostSectionEvent>) {
    events.push(PostSectionEvent::StartSection {
        anchor: node.anchor_slug.clone(),
        level: node.level,
        heading_html: node.heading_html.clone(),
        body_html: node.body_html.clone(),
    });

    if !node.children.is_empty() {
        events.push(PostSectionEvent::StartChildren);
        for child in &node.children {
            append_section_events(child, events);
        }
        events.push(PostSectionEvent::EndChildren);
    }

    events.push(PostSectionEvent::EndSection);
}

fn build_post_toc_view(nodes: &[PostSectionNode]) -> Option<PostTocView> {
    if nodes.is_empty() {
        return None;
    }

    let mut events = Vec::new();
    append_toc_events(nodes, &mut events);
    Some(PostTocView { events })
}

fn append_toc_events(nodes: &[PostSectionNode], events: &mut Vec<PostTocEvent>) {
    events.push(PostTocEvent::StartList);

    for node in nodes {
        let title = node.heading_text.trim().to_string();
        events.push(PostTocEvent::StartItem {
            anchor: node.anchor_slug.clone(),
            title,
            level: node.level,
        });

        if !node.children.is_empty() {
            append_toc_events(&node.children, events);
        }

        events.push(PostTocEvent::EndItem);
    }

    events.push(PostTocEvent::EndList);
}

pub fn build_datastar_append_response(
    payload: AppendPayload,
    load_more_query: String,
) -> Result<Response, HttpError> {
    let AppendPayload {
        offset,
        cards,
        next_cursor,
        total_visible,
    } = payload;

    let appended_count = cards.len();

    let cards_html = if appended_count > 0 {
        let template = PostCardsAppendTemplate {
            posts: cards,
            offset,
        };
        Some(template.render().map_err(|err| {
            HttpError::from(TemplateRenderError::new(
                "application::feed::build_datastar_append_response",
                "Template rendering failed",
                err,
            ))
        })?)
    } else {
        None
    };

    let loader_html = FeedLoaderTemplate {
        view: FeedLoaderContext {
            has_results: total_visible > 0,
            next_cursor,
            load_more_query,
        },
    }
    .render()
    .map_err(|err| {
        HttpError::from(TemplateRenderError::new(
            "application::feed::build_datastar_append_response",
            "Template rendering failed",
            err,
        ))
    })?;

    let mut stream = StreamBuilder::new();

    if let Some(html) = cards_html {
        stream.push_patch(html, "#post-grid", ElementPatchMode::Append);
    }

    stream.push_patch(
        loader_html,
        "#feed-sentinel-container",
        ElementPatchMode::Inner,
    );

    let script = format!(
        "(function() {{ const grid = document.querySelector('#post-grid'); if (grid) {{ grid.setAttribute('data-count', '{}'); }} }})();",
        total_visible
    );
    stream.push_script(script);

    stream.push_signals(r#"{"feedLoading": false}"#);

    Ok(stream.into_response())
}

fn homepage_page_limit(settings: &SiteSettingsRecord) -> u32 {
    let clamped = settings.homepage_size.clamp(1, 48) as u32;
    if clamped == 0 {
        DEFAULT_PAGE_SIZE as u32
    } else {
        clamped
    }
}

pub(crate) fn order_tags_with_pins(counts: &[TagWithCount]) -> Vec<&TagWithCount> {
    let mut ordered: Vec<&TagWithCount> = counts.iter().collect();
    ordered.sort_by(|left, right| {
        right
            .pinned
            .cmp(&left.pinned)
            .then(right.count.cmp(&left.count))
            .then(left.name.to_lowercase().cmp(&right.name.to_lowercase()))
            .then(left.slug.to_lowercase().cmp(&right.slug.to_lowercase()))
    });
    ordered
}

fn build_tag_summaries(
    counts: &[TagWithCount],
    active_tag: Option<&str>,
    total_posts: u64,
    settings: &SiteSettingsRecord,
) -> Vec<views::TagSummary> {
    let mut summaries = Vec::with_capacity(counts.len() + 1);
    summaries.push(views::TagSummary {
        label: "All tags".to_string(),
        path: "/".to_string(),
        count: usize::try_from(total_posts).unwrap_or(usize::MAX),
        is_active: active_tag.is_none(),
    });

    let ordered = order_tags_with_pins(counts);
    let limit = settings.tag_filter_limit.max(0) as usize;
    let mut non_pinned_added = 0;

    for entry in ordered {
        if !entry.pinned && non_pinned_added >= limit {
            continue;
        }

        if !entry.pinned {
            non_pinned_added += 1;
        }

        summaries.push(views::TagSummary {
            label: format!("#{}", entry.name),
            path: format!("/tags/{}", entry.slug),
            count: usize::try_from(entry.count).unwrap_or(usize::MAX),
            is_active: active_tag.map(|tag| tag == entry.slug).unwrap_or(false),
        });
    }

    summaries
}

fn build_month_summaries(
    counts: &[posts::MonthCount],
    active: Option<&str>,
    total_posts: u64,
    limit: i32,
) -> Vec<views::MonthSummary> {
    let mut summaries = Vec::with_capacity(counts.len() + 1);
    summaries.push(views::MonthSummary {
        label: "All months".to_string(),
        path: "/".to_string(),
        count: usize::try_from(total_posts).unwrap_or(usize::MAX),
        is_active: active.is_none(),
    });

    let quota = limit.max(0) as usize;
    for entry in counts.iter().take(quota) {
        summaries.push(views::MonthSummary {
            label: entry.label.clone(),
            path: format!("/months/{}", entry.key),
            count: entry.count,
            is_active: active.map(|value| value == entry.key).unwrap_or(false),
        });
    }

    summaries
}
