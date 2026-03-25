use async_trait::async_trait;
use axum::body::Body;
use http_body_util::BodyExt;
use std::collections::{BTreeMap, BTreeSet, HashMap, hash_map::DefaultHasher};
use std::hash::{Hash, Hasher};
use std::sync::Arc;
use time::{OffsetDateTime, macros::time};
use uuid::Uuid;

use soffio::application::chrome::ChromeService;
pub use soffio::application::feed as feed_render;
pub use soffio::application::feed::{FeedFilter, FeedService};
use soffio::application::page::PageService;
use soffio::application::pagination::{
    CursorPage, NavigationCursor, PageCursor, PageRequest, PostCursor, TagCursor,
};
use soffio::application::repos::{
    NavigationQueryFilter, NavigationRepo, PageQueryFilter, PagesRepo, PostListScope,
    PostQueryFilter, PostTagCount, PostsRepo, RepoError, SectionsRepo, SettingsRepo, TagListRecord,
    TagQueryFilter, TagWithCount, TagsRepo,
};
use soffio::domain::entities::{
    NavigationItemRecord, PageRecord, PostRecord, PostSectionRecord, SiteSettingsRecord, TagRecord,
};
use soffio::domain::types::{NavigationDestinationType, PageStatus, PostStatus};
pub use soffio::domain::{navigation, pages, posts};
use soffio::presentation::views::LayoutContext;

pub fn feed_service() -> FeedService {
    let repo = Arc::new(StaticContentRepo::new());
    FeedService::new(repo.clone(), repo.clone(), repo.clone(), repo, None)
}

pub fn chrome_service() -> ChromeService {
    let repo = Arc::new(StaticContentRepo::new());
    ChromeService::new(repo.clone(), repo, None)
}

pub fn page_service() -> PageService {
    let repo = Arc::new(StaticContentRepo::new());
    PageService::new(repo, None)
}

pub async fn apply_layout<T>(content: T) -> LayoutContext<T> {
    let chrome = chrome_service();
    let layout = chrome.load().await.expect("load chrome");
    LayoutContext::new(layout, content)
}

pub async fn body_to_string(body: Body) -> String {
    let bytes = body.collect().await.expect("collect body").to_bytes();
    String::from_utf8(bytes.to_vec()).expect("utf8 body")
}

#[derive(Clone, Default)]
pub struct StaticContentRepo;

impl StaticContentRepo {
    pub fn new() -> Self {
        Self
    }

    fn all_posts(&self) -> Vec<&'static posts::Post> {
        posts::all().iter().collect()
    }

    fn filtered_posts(&self, filter: &PostQueryFilter) -> Vec<&'static posts::Post> {
        self.all_posts()
            .into_iter()
            .filter(|post| {
                if let Some(search) = filter.search.as_ref() {
                    let needle = search.to_lowercase();
                    let title = post.title.to_lowercase();
                    let excerpt = post.excerpt.to_lowercase();
                    if !title.contains(&needle)
                        && !post.slug.contains(search)
                        && !excerpt.contains(&needle)
                    {
                        return false;
                    }
                }
                true
            })
            .filter(|post| match (&filter.tag, &filter.month) {
                (Some(tag), Some(month)) => {
                    post.tags.contains(&tag.as_str()) && posts::month_key_for(post.date) == *month
                }
                (Some(tag), None) => post.tags.contains(&tag.as_str()),
                (None, Some(month)) => posts::month_key_for(post.date) == *month,
                (None, None) => true,
            })
            .collect()
    }

    fn sorted_posts(
        &self,
        scope: PostListScope,
        filter: &PostQueryFilter,
    ) -> Vec<&'static posts::Post> {
        let mut posts = match scope {
            PostListScope::Public => self
                .filtered_posts(filter)
                .into_iter()
                .filter(|post| post.date <= OffsetDateTime::now_utc().date())
                .collect::<Vec<_>>(),
            PostListScope::Admin { status } => match status {
                Some(PostStatus::Published) | None => self.filtered_posts(filter),
                _ => Vec::new(),
            },
        };
        posts.sort_by_key(|post| std::cmp::Reverse(post.date));
        posts
    }

    fn post_uuid(slug: &str) -> Uuid {
        Self::deterministic_uuid(&["post", slug])
    }

    pub fn record_for(post: &posts::Post) -> PostRecord {
        let published = post.date.with_time(time!(00:00:00)).assume_utc();
        PostRecord {
            id: Self::post_uuid(post.slug),
            slug: post.slug.to_string(),
            title: post.title.to_string(),
            excerpt: post.excerpt.to_string(),
            body_markdown: String::new(),
            status: PostStatus::Published,
            pinned: false,
            scheduled_at: None,
            published_at: Some(published),
            archived_at: None,
            summary_markdown: post.summary.map(|items| items.join("\n")),
            summary_html: post.summary.map(|items| {
                let items = items
                    .iter()
                    .map(|item| format!("<li>{}</li>", item))
                    .collect::<Vec<_>>()
                    .join("");
                format!("<ul>{}</ul>", items)
            }),
            created_at: published,
            updated_at: published,
        }
    }

    fn sections_for(post: &posts::Post) -> Vec<PostSectionRecord> {
        let published = post.date.with_time(time!(00:00:00)).assume_utc();
        let post_id = Self::post_uuid(post.slug);
        let mut parent_stack: Vec<(u8, Uuid)> = Vec::new();
        let mut counters: HashMap<Option<Uuid>, i32> = HashMap::new();
        let mut records = Vec::with_capacity(post.sections.len());

        for section in post.sections.iter() {
            let level = section.level;
            while let Some(&(stack_level, _)) = parent_stack.last() {
                if stack_level < level {
                    break;
                }
                parent_stack.pop();
            }
            let parent = parent_stack.last().map(|&(_, id)| id);
            let counter = counters.entry(parent).or_insert(0);
            *counter += 1;

            let record = PostSectionRecord {
                id: Uuid::new_v4(),
                post_id,
                position: *counter,
                level: level as i16,
                parent_id: parent,
                heading_html: format!("<h{}>{}</h{}>", level, section.title, level),
                heading_text: section.title.to_string(),
                body_html: String::new(),
                contains_code: false,
                contains_math: false,
                contains_mermaid: false,
                anchor_slug: section.id.to_string(),
                created_at: published,
            };
            records.push(record);
            parent_stack.push((level, records.last().unwrap().id));
        }

        records
    }

    fn navigation_records(&self) -> Vec<NavigationItemRecord> {
        let now = OffsetDateTime::UNIX_EPOCH;
        navigation::navigation()
            .entries()
            .iter()
            .map(|entry| match &entry.destination {
                navigation::NavDestination::Internal { slug, .. } => NavigationItemRecord {
                    id: Self::deterministic_uuid(&["nav", entry.label.as_str()]),
                    label: entry.label.clone(),
                    destination_type: NavigationDestinationType::Internal,
                    destination_page_id: Some(Self::deterministic_uuid(&["page", slug.as_str()])),
                    destination_page_slug: Some(slug.as_str().to_string()),
                    destination_url: None,
                    sort_order: entry.order.into(),
                    visible: true,
                    open_in_new_tab: false,
                    created_at: now,
                    updated_at: now,
                },
                navigation::NavDestination::External { url, target } => NavigationItemRecord {
                    id: Self::deterministic_uuid(&["nav", entry.label.as_str()]),
                    label: entry.label.clone(),
                    destination_type: NavigationDestinationType::External,
                    destination_page_id: None,
                    destination_page_slug: None,
                    destination_url: Some(url.to_string()),
                    sort_order: entry.order.into(),
                    visible: true,
                    open_in_new_tab: matches!(target, navigation::LinkTarget::Blank),
                    created_at: now,
                    updated_at: now,
                },
            })
            .collect()
    }

    fn tag_records(&self) -> Vec<TagRecord> {
        let mut tags: BTreeSet<&str> = BTreeSet::new();
        for post in posts::all() {
            for tag in post.tags {
                tags.insert(tag);
            }
        }
        tags.into_iter().map(|slug| self.tag_record(slug)).collect()
    }

    fn tag_record(&self, slug: &str) -> TagRecord {
        let now = OffsetDateTime::now_utc();
        TagRecord {
            id: Self::deterministic_uuid(&["tag", slug]),
            slug: slug.to_string(),
            name: slug.to_string(),
            description: None,
            pinned: false,
            created_at: now,
            updated_at: now,
        }
    }

    fn page_records(&self) -> Vec<PageRecord> {
        let slugs = ["about", "systems-handbook"];
        slugs
            .iter()
            .filter_map(|slug| pages::pages().find_by_slug(slug))
            .map(|page| PageRecord {
                id: Self::deterministic_uuid(&["page", page.slug.as_str()]),
                slug: page.slug.as_str().to_string(),
                title: page.slug.as_str().to_string(),
                body_markdown: page.content_html.clone(),
                rendered_html: page.content_html.clone(),
                status: PageStatus::Published,
                scheduled_at: None,
                published_at: Some(OffsetDateTime::UNIX_EPOCH),
                archived_at: None,
                created_at: OffsetDateTime::UNIX_EPOCH,
                updated_at: OffsetDateTime::UNIX_EPOCH,
            })
            .collect()
    }

    fn deterministic_uuid(parts: &[&str]) -> Uuid {
        let mut hasher_high = DefaultHasher::new();
        for part in parts {
            part.hash(&mut hasher_high);
        }
        let high = hasher_high.finish();

        let mut hasher_low = DefaultHasher::new();
        "soffio".hash(&mut hasher_low);
        for part in parts {
            (part.len() as u64).hash(&mut hasher_low);
        }
        let low = hasher_low.finish();

        let mut bytes = [0u8; 16];
        bytes[..8].copy_from_slice(&high.to_be_bytes());
        bytes[8..].copy_from_slice(&low.to_be_bytes());
        Uuid::from_bytes(bytes)
    }
}

mod navigation_repo;
mod pages_repo;
mod posts_repo;
mod sections_repo;
mod settings_repo;
mod tags_repo;
