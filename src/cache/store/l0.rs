use uuid::Uuid;

use crate::application::pagination::CursorPage;
use crate::domain::api_keys::ApiKeyRecord;
use crate::domain::entities::{NavigationItemRecord, PageRecord, PostRecord, SiteSettingsRecord};
use crate::domain::posts::MonthCount;

use super::{CacheConfig, L0Store, SOURCE, record_l0_evict, record_l0_lookup, rw_read, rw_write};

impl L0Store {
    /// Create a new L0 store with the given configuration.
    pub fn new(config: &CacheConfig) -> Self {
        Self {
            site_settings: std::sync::RwLock::new(None),
            navigation: std::sync::RwLock::new(None),
            tag_counts: std::sync::RwLock::new(None),
            month_counts: std::sync::RwLock::new(None),
            posts_by_id: std::sync::RwLock::new(lru::LruCache::new(
                config.l0_post_limit_non_zero(),
            )),
            posts_by_slug: std::sync::RwLock::new(lru::LruCache::new(
                config.l0_post_limit_non_zero(),
            )),
            pages_by_id: std::sync::RwLock::new(lru::LruCache::new(
                config.l0_page_limit_non_zero(),
            )),
            pages_by_slug: std::sync::RwLock::new(lru::LruCache::new(
                config.l0_page_limit_non_zero(),
            )),
            api_keys_by_prefix: std::sync::RwLock::new(lru::LruCache::new(
                config.l0_api_key_limit_non_zero(),
            )),
            post_lists: std::sync::RwLock::new(lru::LruCache::new(
                config.l0_post_list_limit_non_zero(),
            )),
        }
    }

    pub fn get_site_settings(&self) -> Option<SiteSettingsRecord> {
        let settings = rw_read(&self.site_settings, SOURCE, "get_site_settings").clone();
        record_l0_lookup("site_settings", settings.is_some());
        settings
    }

    pub fn set_site_settings(&self, value: SiteSettingsRecord) {
        *rw_write(&self.site_settings, SOURCE, "set_site_settings") = Some(value);
    }

    pub fn invalidate_site_settings(&self) {
        *rw_write(&self.site_settings, SOURCE, "invalidate_site_settings") = None;
    }

    pub fn get_navigation(&self) -> Option<Vec<NavigationItemRecord>> {
        let navigation = rw_read(&self.navigation, SOURCE, "get_navigation").clone();
        record_l0_lookup("navigation", navigation.is_some());
        navigation
    }

    pub fn set_navigation(&self, value: Vec<NavigationItemRecord>) {
        *rw_write(&self.navigation, SOURCE, "set_navigation") = Some(value);
    }

    pub fn invalidate_navigation(&self) {
        *rw_write(&self.navigation, SOURCE, "invalidate_navigation") = None;
    }

    pub fn get_tag_counts(&self) -> Option<Vec<crate::application::repos::TagWithCount>> {
        let tags = rw_read(&self.tag_counts, SOURCE, "get_tag_counts").clone();
        record_l0_lookup("tag_counts", tags.is_some());
        tags
    }

    pub fn set_tag_counts(&self, value: Vec<crate::application::repos::TagWithCount>) {
        *rw_write(&self.tag_counts, SOURCE, "set_tag_counts") = Some(value);
    }

    pub fn invalidate_tag_counts(&self) {
        *rw_write(&self.tag_counts, SOURCE, "invalidate_tag_counts") = None;
    }

    pub fn get_month_counts(&self) -> Option<Vec<MonthCount>> {
        let months = rw_read(&self.month_counts, SOURCE, "get_month_counts").clone();
        record_l0_lookup("month_counts", months.is_some());
        months
    }

    pub fn set_month_counts(&self, value: Vec<MonthCount>) {
        *rw_write(&self.month_counts, SOURCE, "set_month_counts") = Some(value);
    }

    pub fn invalidate_month_counts(&self) {
        *rw_write(&self.month_counts, SOURCE, "invalidate_month_counts") = None;
    }

    pub fn get_post_by_id(&self, id: Uuid) -> Option<PostRecord> {
        let post = rw_write(&self.posts_by_id, SOURCE, "get_post_by_id")
            .get(&id)
            .cloned();
        record_l0_lookup("post_by_id", post.is_some());
        post
    }

    pub fn get_post_by_slug(&self, slug: &str) -> Option<PostRecord> {
        let post = rw_write(&self.posts_by_slug, SOURCE, "get_post_by_slug")
            .get(slug)
            .cloned();
        record_l0_lookup("post_by_slug", post.is_some());
        post
    }

    pub fn set_post(&self, post: PostRecord) {
        let mut by_id = rw_write(&self.posts_by_id, SOURCE, "set_post.by_id");
        let mut by_slug = rw_write(&self.posts_by_slug, SOURCE, "set_post.by_slug");
        let existed_by_id = by_id.contains(&post.id);
        if by_id.push(post.id, post.clone()).is_some() && !existed_by_id {
            record_l0_evict("post_by_id");
        }

        let slug = post.slug.clone();
        let existed_by_slug = by_slug.contains(&slug);
        if by_slug.push(slug, post).is_some() && !existed_by_slug {
            record_l0_evict("post_by_slug");
        }
    }

    pub fn invalidate_post(&self, id: Uuid, slug: &str) {
        rw_write(&self.posts_by_id, SOURCE, "invalidate_post.by_id").pop(&id);
        rw_write(&self.posts_by_slug, SOURCE, "invalidate_post.by_slug").pop(slug);
    }

    pub fn get_page_by_id(&self, id: Uuid) -> Option<PageRecord> {
        let page = rw_write(&self.pages_by_id, SOURCE, "get_page_by_id")
            .get(&id)
            .cloned();
        record_l0_lookup("page_by_id", page.is_some());
        page
    }

    pub fn get_page_by_slug(&self, slug: &str) -> Option<PageRecord> {
        let page = rw_write(&self.pages_by_slug, SOURCE, "get_page_by_slug")
            .get(slug)
            .cloned();
        record_l0_lookup("page_by_slug", page.is_some());
        page
    }

    pub fn set_page(&self, page: PageRecord) {
        let mut by_id = rw_write(&self.pages_by_id, SOURCE, "set_page.by_id");
        let mut by_slug = rw_write(&self.pages_by_slug, SOURCE, "set_page.by_slug");
        let existed_by_id = by_id.contains(&page.id);
        if by_id.push(page.id, page.clone()).is_some() && !existed_by_id {
            record_l0_evict("page_by_id");
        }

        let slug = page.slug.clone();
        let existed_by_slug = by_slug.contains(&slug);
        if by_slug.push(slug, page).is_some() && !existed_by_slug {
            record_l0_evict("page_by_slug");
        }
    }

    pub fn invalidate_page(&self, id: Uuid, slug: &str) {
        rw_write(&self.pages_by_id, SOURCE, "invalidate_page.by_id").pop(&id);
        rw_write(&self.pages_by_slug, SOURCE, "invalidate_page.by_slug").pop(slug);
    }

    pub fn get_api_key_by_prefix(&self, prefix: &str) -> Option<ApiKeyRecord> {
        let key = rw_write(&self.api_keys_by_prefix, SOURCE, "get_api_key_by_prefix")
            .get(prefix)
            .cloned();
        record_l0_lookup("api_key", key.is_some());
        key
    }

    pub fn set_api_key(&self, key: ApiKeyRecord) {
        let mut keys = rw_write(&self.api_keys_by_prefix, SOURCE, "set_api_key");
        let prefix = key.prefix.clone();
        let existed = keys.contains(&prefix);
        if keys.push(prefix, key).is_some() && !existed {
            record_l0_evict("api_key");
        }
    }

    pub fn invalidate_api_key(&self, prefix: &str) {
        rw_write(&self.api_keys_by_prefix, SOURCE, "invalidate_api_key").pop(prefix);
    }

    pub fn get_post_list(
        &self,
        filter_hash: u64,
        cursor_hash: u64,
    ) -> Option<CursorPage<PostRecord>> {
        let page = rw_write(&self.post_lists, SOURCE, "get_post_list")
            .get(&(filter_hash, cursor_hash))
            .cloned();
        record_l0_lookup("post_list", page.is_some());
        page
    }

    pub fn set_post_list(&self, filter_hash: u64, cursor_hash: u64, page: CursorPage<PostRecord>) {
        let mut post_lists = rw_write(&self.post_lists, SOURCE, "set_post_list");
        let key = (filter_hash, cursor_hash);
        let existed = post_lists.contains(&key);
        if post_lists.push(key, page).is_some() && !existed {
            record_l0_evict("post_list");
        }
    }

    pub fn invalidate_all_post_lists(&self) {
        rw_write(&self.post_lists, SOURCE, "invalidate_all_post_lists").clear();
    }

    /// Clear all cached data.
    pub fn clear(&self) {
        self.invalidate_site_settings();
        self.invalidate_navigation();
        self.invalidate_tag_counts();
        self.invalidate_month_counts();
        rw_write(&self.posts_by_id, SOURCE, "clear.posts_by_id").clear();
        rw_write(&self.posts_by_slug, SOURCE, "clear.posts_by_slug").clear();
        rw_write(&self.pages_by_id, SOURCE, "clear.pages_by_id").clear();
        rw_write(&self.pages_by_slug, SOURCE, "clear.pages_by_slug").clear();
        rw_write(&self.api_keys_by_prefix, SOURCE, "clear.api_keys_by_prefix").clear();
        rw_write(&self.post_lists, SOURCE, "clear.post_lists").clear();
    }
}
