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

#[async_trait]
impl PostsRepo for StaticContentRepo {
    async fn list_posts(
        &self,
        scope: PostListScope,
        filter: &PostQueryFilter,
        page: PageRequest<PostCursor>,
    ) -> Result<CursorPage<PostRecord>, RepoError> {
        let posts = self.sorted_posts(scope, filter);
        let limit = page.limit.clamp(1, 100) as usize;

        let start = page
            .cursor
            .as_ref()
            .and_then(|cursor| {
                posts
                    .iter()
                    .position(|post| Self::post_uuid(post.slug) == cursor.id())
                    .map(|idx| idx + 1)
            })
            .unwrap_or(0);

        let slice = posts
            .iter()
            .skip(start)
            .take(limit)
            .cloned()
            .collect::<Vec<_>>();
        let records: Vec<PostRecord> = slice.iter().map(|post| Self::record_for(post)).collect();

        let next_cursor = posts.get(start + limit).map(|post| {
            PostCursor::published(
                post.date.with_time(time!(00:00:00)).assume_utc(),
                Self::post_uuid(post.slug),
                false,
            )
            .encode()
        });

        Ok(CursorPage::new(records, next_cursor))
    }

    async fn find_by_slug(&self, slug: &str) -> Result<Option<PostRecord>, RepoError> {
        Ok(posts::find_by_slug(slug).map(Self::record_for))
    }

    async fn find_by_id(&self, id: Uuid) -> Result<Option<PostRecord>, RepoError> {
        Ok(self
            .all_posts()
            .into_iter()
            .find(|post| Self::post_uuid(post.slug) == id)
            .map(Self::record_for))
    }

    async fn list_month_counts(
        &self,
        scope: PostListScope,
        filter: &PostQueryFilter,
    ) -> Result<Vec<posts::MonthCount>, RepoError> {
        let posts: Vec<&posts::Post> = self.sorted_posts(scope, filter);

        let mut counts: BTreeMap<String, (String, usize)> = BTreeMap::new();
        for post in posts {
            let key = posts::month_key_for(post.date);
            let label = posts::month_label_for(post.date);
            counts
                .entry(key)
                .and_modify(|entry| entry.1 += 1)
                .or_insert((label, 1));
        }

        let mut months = counts
            .into_iter()
            .map(|(key, (label, count))| posts::MonthCount { key, label, count })
            .collect::<Vec<_>>();
        months.sort_by(|a, b| b.key.cmp(&a.key));
        Ok(months)
    }

    async fn count_posts(
        &self,
        scope: PostListScope,
        filter: &PostQueryFilter,
    ) -> Result<u64, RepoError> {
        let posts = self.sorted_posts(scope, filter);
        Ok(posts.len() as u64)
    }

    async fn count_posts_before(
        &self,
        scope: PostListScope,
        filter: &PostQueryFilter,
        cursor: &PostCursor,
    ) -> Result<u64, RepoError> {
        let posts = self.sorted_posts(scope, filter);
        let idx = posts
            .iter()
            .position(|post| Self::post_uuid(post.slug) == cursor.id())
            .unwrap_or(0);
        Ok(idx as u64)
    }

    async fn list_tag_counts(
        &self,
        scope: PostListScope,
        filter: &PostQueryFilter,
    ) -> Result<Vec<PostTagCount>, RepoError> {
        let posts = self.sorted_posts(scope, filter);
        let mut counts: BTreeMap<&str, u64> = BTreeMap::new();
        for post in posts {
            for tag in post.tags {
                *counts.entry(tag).or_default() += 1;
            }
        }
        Ok(counts
            .into_iter()
            .map(|(slug, count)| PostTagCount {
                slug: slug.to_string(),
                name: slug.to_string(),
                count,
            })
            .collect())
    }
}

#[async_trait]
impl SectionsRepo for StaticContentRepo {
    async fn list_sections(&self, post_id: Uuid) -> Result<Vec<PostSectionRecord>, RepoError> {
        let post = self
            .all_posts()
            .into_iter()
            .find(|post| Self::post_uuid(post.slug) == post_id)
            .expect("post exists");
        Ok(Self::sections_for(post))
    }
}

#[async_trait]
impl TagsRepo for StaticContentRepo {
    async fn list_all(&self) -> Result<Vec<TagRecord>, RepoError> {
        Ok(self.tag_records())
    }

    async fn find_by_slug(&self, slug: &str) -> Result<Option<TagRecord>, RepoError> {
        Ok(self.tag_records().into_iter().find(|tag| tag.slug == slug))
    }

    async fn count_usage(&self, id: Uuid) -> Result<u64, RepoError> {
        let slug = self
            .tag_records()
            .into_iter()
            .find(|tag| tag.id == id)
            .map(|t| t.slug)
            .unwrap_or_default();
        Ok(self
            .all_posts()
            .into_iter()
            .filter(|post| post.tags.contains(&slug.as_str()))
            .count() as u64)
    }

    async fn list_admin_tags(
        &self,
        _pinned: Option<bool>,
        filter: &TagQueryFilter,
        page: PageRequest<TagCursor>,
    ) -> Result<CursorPage<TagListRecord>, RepoError> {
        let mut tags = self.tag_records();

        if let Some(search) = &filter.search {
            let needle = search.to_lowercase();
            tags.retain(|tag| {
                tag.slug.contains(&needle) || tag.name.to_lowercase().contains(&needle)
            });
        }

        let start = page
            .cursor
            .as_ref()
            .and_then(|cursor| tags.iter().position(|tag| tag.id == cursor.id()))
            .map(|idx| idx + 1)
            .unwrap_or(0);

        let limit = page.limit.clamp(1, 100) as usize;
        let slice = tags.iter().skip(start).take(limit);
        let records: Vec<TagListRecord> = slice
            .map(|tag| TagListRecord {
                id: tag.id,
                slug: tag.slug.clone(),
                name: tag.name.clone(),
                description: tag.description.clone(),
                pinned: tag.pinned,
                usage_count: self
                    .all_posts()
                    .into_iter()
                    .filter(|post| post.tags.contains(&tag.slug.as_str()))
                    .count() as u64,
                primary_time: tag.created_at,
                updated_at: Some(tag.updated_at),
                created_at: tag.created_at,
            })
            .collect();

        let next_cursor = tags
            .get(start + limit)
            .map(|tag| TagCursor::new(tag.pinned, tag.created_at, tag.id).encode());

        Ok(CursorPage::new(records, next_cursor))
    }

    async fn count_tags(
        &self,
        _pinned: Option<bool>,
        filter: &TagQueryFilter,
    ) -> Result<u64, RepoError> {
        let mut tags = self.tag_records();
        if let Some(search) = &filter.search {
            let needle = search.to_lowercase();
            tags.retain(|tag| {
                tag.slug.contains(&needle) || tag.name.to_lowercase().contains(&needle)
            });
        }
        Ok(tags.len() as u64)
    }

    async fn month_counts(
        &self,
        _pinned: Option<bool>,
        filter: &TagQueryFilter,
    ) -> Result<Vec<posts::MonthCount>, RepoError> {
        let mut months: BTreeMap<String, (String, usize)> = BTreeMap::new();
        for post in self.filtered_posts(&PostQueryFilter {
            tag: filter.search.clone(),
            ..PostQueryFilter::default()
        }) {
            let key = posts::month_key_for(post.date);
            let label = posts::month_label_for(post.date);
            months
                .entry(key)
                .and_modify(|entry| entry.1 += 1)
                .or_insert((label, 1));
        }

        let mut items = months
            .into_iter()
            .map(|(key, (label, count))| posts::MonthCount { key, label, count })
            .collect::<Vec<_>>();
        items.sort_by(|a, b| b.key.cmp(&a.key));
        Ok(items)
    }
    async fn find_by_id(&self, id: Uuid) -> Result<Option<TagRecord>, RepoError> {
        Ok(self.tag_records().into_iter().find(|tag| tag.id == id))
    }

    async fn list_for_post(&self, post_id: Uuid) -> Result<Vec<TagRecord>, RepoError> {
        let post = self
            .all_posts()
            .into_iter()
            .find(|post| Self::post_uuid(post.slug) == post_id)
            .expect("post exists");
        Ok(post.tags.iter().map(|slug| self.tag_record(slug)).collect())
    }

    async fn list_with_counts(&self) -> Result<Vec<TagWithCount>, RepoError> {
        let mut counts: BTreeMap<&str, i64> = BTreeMap::new();
        for post in posts::all() {
            for tag in post.tags {
                *counts.entry(tag).or_default() += 1;
            }
        }
        Ok(counts
            .into_iter()
            .map(|(slug, count)| TagWithCount {
                id: Self::deterministic_uuid(&["tag", slug]),
                slug: slug.to_string(),
                name: slug.to_string(),
                pinned: false,
                count,
            })
            .collect())
    }
}

#[async_trait]
impl SettingsRepo for StaticContentRepo {
    async fn load_site_settings(&self) -> Result<SiteSettingsRecord, RepoError> {
        Ok(SiteSettingsRecord {
            homepage_size: 6,
            admin_page_size: 20,
            show_tag_aggregations: true,
            show_month_aggregations: true,
            tag_filter_limit: 16,
            month_filter_limit: 16,
            global_toc_enabled: true,
            brand_title: "Soffio".to_string(),
            brand_href: "/".to_string(),
            footer_copy: "Stillness guides the wind; the wind reshapes stillness.".to_string(),
            public_site_url: "http://localhost:3000/".to_string(),
            favicon_svg: "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\"></svg>"
                .to_string(),
            timezone: chrono_tz::Asia::Shanghai,
            meta_title: "Soffio".to_string(),
            meta_description: "Whispers on motion, balance, and form.".to_string(),
            og_title: "Soffio".to_string(),
            og_description: "Traces of motion, balance, and form in continual drift.".to_string(),
            updated_at: OffsetDateTime::UNIX_EPOCH,
        })
    }

    async fn upsert_site_settings(&self, _settings: SiteSettingsRecord) -> Result<(), RepoError> {
        Ok(())
    }
}

#[async_trait]
impl NavigationRepo for StaticContentRepo {
    async fn list_navigation(
        &self,
        _visibility: Option<bool>,
        filter: &NavigationQueryFilter,
        page: PageRequest<NavigationCursor>,
    ) -> Result<CursorPage<NavigationItemRecord>, RepoError> {
        let mut records = self.navigation_records();
        if let Some(search) = filter.search.as_ref() {
            let needle = search.to_lowercase();
            records.retain(|r| r.label.to_lowercase().contains(&needle));
        }

        let start = page
            .cursor
            .as_ref()
            .and_then(|cursor| records.iter().position(|nav| nav.id == cursor.id()))
            .map(|idx| idx + 1)
            .unwrap_or(0);
        let limit = page.limit.clamp(1, 100) as usize;
        let slice = records
            .iter()
            .skip(start)
            .take(limit)
            .cloned()
            .collect::<Vec<_>>();
        let next_cursor = records
            .get(start + limit)
            .map(|nav| NavigationCursor::new(nav.sort_order, nav.created_at, nav.id).encode());

        Ok(CursorPage::new(slice, next_cursor))
    }

    async fn count_navigation(
        &self,
        _visibility: Option<bool>,
        filter: &NavigationQueryFilter,
    ) -> Result<u64, RepoError> {
        let mut records = self.navigation_records();
        if let Some(search) = filter.search.as_ref() {
            let needle = search.to_lowercase();
            records.retain(|r| r.label.to_lowercase().contains(&needle));
        }
        Ok(records.len() as u64)
    }

    async fn count_external_navigation(
        &self,
        _visibility: Option<bool>,
        filter: &NavigationQueryFilter,
    ) -> Result<u64, RepoError> {
        let mut records = self.navigation_records();
        if let Some(search) = filter.search.as_ref() {
            let needle = search.to_lowercase();
            records.retain(|r| r.label.to_lowercase().contains(&needle));
        }
        Ok(records
            .into_iter()
            .filter(|record| record.destination_type == NavigationDestinationType::External)
            .count() as u64)
    }

    async fn find_by_id(&self, id: Uuid) -> Result<Option<NavigationItemRecord>, RepoError> {
        Ok(self
            .navigation_records()
            .into_iter()
            .find(|record| record.id == id))
    }
}

#[async_trait]
impl PagesRepo for StaticContentRepo {
    async fn list_pages(
        &self,
        _status: Option<PageStatus>,
        limit: u32,
        cursor: Option<PageCursor>,
        _filter: &PageQueryFilter,
    ) -> Result<CursorPage<PageRecord>, RepoError> {
        let mut pages = self.page_records();
        pages.sort_by_key(|p| p.created_at);

        let start = cursor
            .as_ref()
            .and_then(|c| pages.iter().position(|p| p.id == c.id()))
            .map(|idx| idx + 1)
            .unwrap_or(0);
        let limit = limit.clamp(1, 100) as usize;
        let slice = pages
            .iter()
            .skip(start)
            .take(limit)
            .cloned()
            .collect::<Vec<_>>();
        let next_cursor = pages
            .get(start + limit)
            .map(|p| PageCursor::new(p.created_at, p.id).encode());

        Ok(CursorPage::new(slice, next_cursor))
    }

    async fn find_by_slug(&self, slug: &str) -> Result<Option<PageRecord>, RepoError> {
        Ok(self
            .page_records()
            .into_iter()
            .find(|record| record.slug == slug))
    }

    async fn find_by_id(&self, id: Uuid) -> Result<Option<PageRecord>, RepoError> {
        Ok(self
            .page_records()
            .into_iter()
            .find(|record| record.id == id))
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
    ) -> Result<Vec<posts::MonthCount>, RepoError> {
        Ok(Vec::new())
    }
}
