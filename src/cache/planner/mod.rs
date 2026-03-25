//! Consumption plan generation.
//!
//! Merges multiple cache events into an optimized execution plan.

use std::collections::{HashMap, HashSet};
use std::fmt;

use uuid::Uuid;

use super::events::{CacheEvent, EventKind};
use super::keys::EntityKey;

#[cfg(test)]
mod tests;

/// Actions to execute for cache consistency.
///
/// The planner merges multiple events into a single plan, deduplicating
/// and keeping only the latest state for each entity.
#[derive(Debug, Default)]
pub struct ConsumptionPlan {
    /// Entities to invalidate from cache.
    pub invalidate_entities: HashSet<EntityKey>,

    /// Whether to warm site settings.
    pub warm_site_settings: bool,
    /// Whether to warm navigation.
    pub warm_navigation: bool,
    /// Whether to warm pages linked from visible navigation.
    pub warm_navigation_pages: bool,
    /// Whether to warm aggregations (tag counts, month counts).
    pub warm_aggregations: bool,
    /// Specific posts to warm by ID.
    pub warm_posts: HashSet<Uuid>,
    /// Specific pages to warm by ID.
    pub warm_pages: HashSet<Uuid>,
    /// Whether to warm the homepage.
    pub warm_homepage: bool,
    /// Whether to warm the RSS/Atom feed.
    pub warm_feed: bool,
    /// Whether to warm the sitemap.
    pub warm_sitemap: bool,
}

impl fmt::Display for ConsumptionPlan {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "ConsumptionPlan {{ invalidate: {}, warm_settings: {}, warm_nav: {}, \
             warm_nav_pages: {}, warm_agg: {}, warm_posts: {}, warm_pages: {}, \
             warm_homepage: {}, warm_feed: {}, warm_sitemap: {} }}",
            self.invalidate_entities.len(),
            self.warm_site_settings,
            self.warm_navigation,
            self.warm_navigation_pages,
            self.warm_aggregations,
            self.warm_posts.len(),
            self.warm_pages.len(),
            self.warm_homepage,
            self.warm_feed,
            self.warm_sitemap,
        )
    }
}

impl ConsumptionPlan {
    /// Merge multiple events into an optimized plan.
    ///
    /// - Deduplicates by event ID
    /// - Groups by entity, keeping latest epoch
    /// - Generates invalidation and warm actions
    pub fn from_events(events: Vec<CacheEvent>) -> Self {
        let mut plan = Self::default();
        let mut seen_ids = HashSet::new();

        // Dedupe by event ID
        let events: Vec<_> = events
            .into_iter()
            .filter(|e| seen_ids.insert(e.id))
            .collect();

        // Track latest event per entity
        let mut post_epochs: HashMap<Uuid, (u64, EventKind)> = HashMap::new();
        let mut page_epochs: HashMap<Uuid, (u64, EventKind)> = HashMap::new();

        for event in events {
            match &event.kind {
                EventKind::SiteSettingsUpdated => {
                    plan.invalidate_entities.insert(EntityKey::SiteSettings);
                    plan.warm_site_settings = true;
                }
                EventKind::NavigationUpdated => {
                    plan.invalidate_entities.insert(EntityKey::Navigation);
                    plan.warm_navigation = true;
                    plan.warm_navigation_pages = true;
                }
                EventKind::PostUpserted { post_id, .. }
                | EventKind::PostDeleted { post_id, .. } => {
                    let entry = post_epochs.entry(*post_id);
                    entry
                        .and_modify(|(e, k)| {
                            if event.epoch > *e {
                                *e = event.epoch;
                                *k = event.kind.clone();
                            }
                        })
                        .or_insert((event.epoch, event.kind.clone()));
                }
                EventKind::PageUpserted { page_id, .. }
                | EventKind::PageDeleted { page_id, .. } => {
                    let entry = page_epochs.entry(*page_id);
                    entry
                        .and_modify(|(e, k)| {
                            if event.epoch > *e {
                                *e = event.epoch;
                                *k = event.kind.clone();
                            }
                        })
                        .or_insert((event.epoch, event.kind.clone()));
                }
                EventKind::TagsChanged => {
                    plan.invalidate_entities.insert(EntityKey::PostAggTags);
                    plan.invalidate_entities.insert(EntityKey::PostsIndex);
                    plan.warm_aggregations = true;
                }
                EventKind::ApiKeyUpserted { prefix } | EventKind::ApiKeyRevoked { prefix } => {
                    plan.invalidate_entities
                        .insert(EntityKey::ApiKey(prefix.clone()));
                }
                EventKind::WarmupOnStartup => {
                    plan.warm_site_settings = true;
                    plan.warm_navigation = true;
                    plan.warm_navigation_pages = true;
                    plan.warm_aggregations = true;
                    plan.warm_homepage = true;
                    plan.warm_feed = true;
                    plan.warm_sitemap = true;
                }
            }
        }

        // Process post events
        let mut any_post_changed = false;
        for (post_id, (_, kind)) in post_epochs {
            any_post_changed = true;
            match kind {
                EventKind::PostDeleted { slug, .. } => {
                    plan.invalidate_entities.insert(EntityKey::Post(post_id));
                    plan.invalidate_entities
                        .insert(EntityKey::PostSlug(slug.clone()));
                }
                EventKind::PostUpserted {
                    slug,
                    previous_slug,
                    ..
                } => {
                    plan.invalidate_entities.insert(EntityKey::Post(post_id));
                    plan.invalidate_entities
                        .insert(EntityKey::PostSlug(slug.clone()));
                    if let Some(previous_slug) = previous_slug
                        && previous_slug != slug
                    {
                        plan.invalidate_entities
                            .insert(EntityKey::PostSlug(previous_slug.clone()));
                    }
                    plan.warm_posts.insert(post_id);
                }
                _ => {}
            }
        }

        // If any post changed, invalidate derived collections
        if any_post_changed {
            plan.invalidate_entities.insert(EntityKey::PostsIndex);
            plan.invalidate_entities.insert(EntityKey::PostAggTags);
            plan.invalidate_entities.insert(EntityKey::PostAggMonths);
            plan.invalidate_entities.insert(EntityKey::Feed);
            plan.invalidate_entities.insert(EntityKey::Sitemap);
            plan.warm_aggregations = true;
            plan.warm_homepage = true;
            plan.warm_feed = true;
            plan.warm_sitemap = true;
        }

        // Process page events
        for (page_id, (_, kind)) in page_epochs {
            match kind {
                EventKind::PageDeleted { slug, .. } => {
                    plan.invalidate_entities.insert(EntityKey::Page(page_id));
                    plan.invalidate_entities
                        .insert(EntityKey::PageSlug(slug.clone()));
                }
                EventKind::PageUpserted {
                    slug,
                    previous_slug,
                    ..
                } => {
                    plan.invalidate_entities.insert(EntityKey::Page(page_id));
                    plan.invalidate_entities
                        .insert(EntityKey::PageSlug(slug.clone()));
                    if let Some(previous_slug) = previous_slug
                        && previous_slug != slug
                    {
                        plan.invalidate_entities
                            .insert(EntityKey::PageSlug(previous_slug.clone()));
                    }
                    plan.warm_pages.insert(page_id);
                }
                _ => {}
            }
            plan.invalidate_entities.insert(EntityKey::Sitemap);
            plan.warm_sitemap = true;
        }

        plan
    }

    /// Check if the plan has any actions to execute.
    pub fn is_empty(&self) -> bool {
        self.invalidate_entities.is_empty()
            && !self.warm_site_settings
            && !self.warm_navigation
            && !self.warm_navigation_pages
            && !self.warm_aggregations
            && self.warm_posts.is_empty()
            && self.warm_pages.is_empty()
            && !self.warm_homepage
            && !self.warm_feed
            && !self.warm_sitemap
    }

    /// Check if the plan has any warm actions to execute.
    pub fn has_warm_actions(&self) -> bool {
        self.warm_site_settings
            || self.warm_navigation
            || self.warm_navigation_pages
            || self.warm_aggregations
            || !self.warm_posts.is_empty()
            || !self.warm_pages.is_empty()
            || self.warm_homepage
            || self.warm_feed
            || self.warm_sitemap
    }
}
