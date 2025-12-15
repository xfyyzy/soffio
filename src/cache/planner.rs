//! Consumption plan generation.
//!
//! Merges multiple cache events into an optimized execution plan.

use std::collections::{HashMap, HashSet};
use std::fmt;

use uuid::Uuid;

use super::events::{CacheEvent, EventKind};
use super::keys::EntityKey;

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
                EventKind::PostUpserted { slug, .. } => {
                    plan.invalidate_entities.insert(EntityKey::Post(post_id));
                    plan.invalidate_entities
                        .insert(EntityKey::PostSlug(slug.clone()));
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
                EventKind::PageUpserted { slug, .. } => {
                    plan.invalidate_entities.insert(EntityKey::Page(page_id));
                    plan.invalidate_entities
                        .insert(EntityKey::PageSlug(slug.clone()));
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cache::events::CacheEvent;

    fn make_event(kind: EventKind, epoch: u64) -> CacheEvent {
        CacheEvent::new(kind, epoch)
    }

    #[test]
    fn site_settings_update() {
        let events = vec![make_event(EventKind::SiteSettingsUpdated, 0)];
        let plan = ConsumptionPlan::from_events(events);

        assert!(plan.invalidate_entities.contains(&EntityKey::SiteSettings));
        assert!(plan.warm_site_settings);
    }

    #[test]
    fn navigation_update() {
        let events = vec![make_event(EventKind::NavigationUpdated, 0)];
        let plan = ConsumptionPlan::from_events(events);

        assert!(plan.invalidate_entities.contains(&EntityKey::Navigation));
        assert!(plan.warm_navigation);
        assert!(plan.warm_navigation_pages);
    }

    #[test]
    fn post_upsert_triggers_derived_invalidation() {
        let post_id = Uuid::new_v4();
        let events = vec![make_event(
            EventKind::PostUpserted {
                post_id,
                slug: "test".to_string(),
            },
            0,
        )];
        let plan = ConsumptionPlan::from_events(events);

        assert!(plan.invalidate_entities.contains(&EntityKey::Post(post_id)));
        assert!(
            plan.invalidate_entities
                .contains(&EntityKey::PostSlug("test".to_string()))
        );
        assert!(plan.invalidate_entities.contains(&EntityKey::PostsIndex));
        assert!(plan.invalidate_entities.contains(&EntityKey::PostAggTags));
        assert!(plan.invalidate_entities.contains(&EntityKey::PostAggMonths));
        assert!(plan.invalidate_entities.contains(&EntityKey::Feed));
        assert!(plan.invalidate_entities.contains(&EntityKey::Sitemap));
        assert!(plan.warm_posts.contains(&post_id));
        assert!(plan.warm_aggregations);
        assert!(plan.warm_homepage);
        assert!(plan.warm_feed);
        assert!(plan.warm_sitemap);
    }

    #[test]
    fn post_delete_does_not_warm_post() {
        let post_id = Uuid::new_v4();
        let events = vec![make_event(
            EventKind::PostDeleted {
                post_id,
                slug: "test".to_string(),
            },
            0,
        )];
        let plan = ConsumptionPlan::from_events(events);

        assert!(plan.invalidate_entities.contains(&EntityKey::Post(post_id)));
        assert!(!plan.warm_posts.contains(&post_id)); // Don't warm deleted post
        assert!(plan.warm_aggregations);
    }

    #[test]
    fn page_upsert() {
        let page_id = Uuid::new_v4();
        let events = vec![make_event(
            EventKind::PageUpserted {
                page_id,
                slug: "about".to_string(),
            },
            0,
        )];
        let plan = ConsumptionPlan::from_events(events);

        assert!(plan.invalidate_entities.contains(&EntityKey::Page(page_id)));
        assert!(
            plan.invalidate_entities
                .contains(&EntityKey::PageSlug("about".to_string()))
        );
        assert!(plan.invalidate_entities.contains(&EntityKey::Sitemap));
        assert!(plan.warm_pages.contains(&page_id));
        assert!(plan.warm_sitemap);
    }

    #[test]
    fn warmup_on_startup() {
        let events = vec![make_event(EventKind::WarmupOnStartup, 0)];
        let plan = ConsumptionPlan::from_events(events);

        assert!(plan.warm_site_settings);
        assert!(plan.warm_navigation);
        assert!(plan.warm_navigation_pages);
        assert!(plan.warm_aggregations);
        assert!(plan.warm_homepage);
        assert!(plan.warm_feed);
        assert!(plan.warm_sitemap);
    }

    #[test]
    fn dedupe_by_event_id() {
        let post_id = Uuid::new_v4();
        let event = make_event(
            EventKind::PostUpserted {
                post_id,
                slug: "test".to_string(),
            },
            0,
        );

        // Same event twice
        let events = vec![event.clone(), event];
        let plan = ConsumptionPlan::from_events(events);

        // Should only have one post to warm
        assert_eq!(plan.warm_posts.len(), 1);
    }

    #[test]
    fn keeps_latest_epoch() {
        let post_id = Uuid::new_v4();

        // First upsert, then delete (delete is latest)
        let events = vec![
            make_event(
                EventKind::PostUpserted {
                    post_id,
                    slug: "test".to_string(),
                },
                0,
            ),
            make_event(
                EventKind::PostDeleted {
                    post_id,
                    slug: "test".to_string(),
                },
                1,
            ),
        ];
        let plan = ConsumptionPlan::from_events(events);

        // Should not warm the deleted post
        assert!(!plan.warm_posts.contains(&post_id));
        assert!(plan.invalidate_entities.contains(&EntityKey::Post(post_id)));
    }

    #[test]
    fn api_key_events() {
        let events = vec![
            make_event(
                EventKind::ApiKeyUpserted {
                    prefix: "sof_test_".to_string(),
                },
                0,
            ),
            make_event(
                EventKind::ApiKeyRevoked {
                    prefix: "sof_old_".to_string(),
                },
                1,
            ),
        ];
        let plan = ConsumptionPlan::from_events(events);

        assert!(
            plan.invalidate_entities
                .contains(&EntityKey::ApiKey("sof_test_".to_string()))
        );
        assert!(
            plan.invalidate_entities
                .contains(&EntityKey::ApiKey("sof_old_".to_string()))
        );
    }

    #[test]
    fn display_format() {
        let plan = ConsumptionPlan::default();
        let display = format!("{}", plan);
        assert!(display.contains("ConsumptionPlan"));
        assert!(display.contains("invalidate: 0"));
    }

    #[test]
    fn is_empty() {
        let plan = ConsumptionPlan::default();
        assert!(plan.is_empty());

        let events = vec![make_event(EventKind::SiteSettingsUpdated, 0)];
        let plan = ConsumptionPlan::from_events(events);
        assert!(!plan.is_empty());
    }
}
