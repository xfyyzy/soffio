use uuid::Uuid;

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
            previous_slug: None,
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
            previous_slug: None,
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
            previous_slug: None,
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
                previous_slug: None,
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
fn tags_changed_invalidates_tag_aggregates_and_post_index() {
    let events = vec![make_event(EventKind::TagsChanged, 0)];
    let plan = ConsumptionPlan::from_events(events);

    assert!(plan.invalidate_entities.contains(&EntityKey::PostAggTags));
    assert!(plan.invalidate_entities.contains(&EntityKey::PostsIndex));
    assert!(plan.warm_aggregations);
}

#[test]
fn post_slug_change_invalidates_previous_slug() {
    let post_id = Uuid::new_v4();
    let events = vec![make_event(
        EventKind::PostUpserted {
            post_id,
            slug: "new-slug".to_string(),
            previous_slug: Some("old-slug".to_string()),
        },
        0,
    )];
    let plan = ConsumptionPlan::from_events(events);

    assert!(
        plan.invalidate_entities
            .contains(&EntityKey::PostSlug("new-slug".to_string()))
    );
    assert!(
        plan.invalidate_entities
            .contains(&EntityKey::PostSlug("old-slug".to_string()))
    );
}

#[test]
fn page_slug_change_invalidates_previous_slug() {
    let page_id = Uuid::new_v4();
    let events = vec![make_event(
        EventKind::PageUpserted {
            page_id,
            slug: "new-page".to_string(),
            previous_slug: Some("old-page".to_string()),
        },
        0,
    )];
    let plan = ConsumptionPlan::from_events(events);

    assert!(
        plan.invalidate_entities
            .contains(&EntityKey::PageSlug("new-page".to_string()))
    );
    assert!(
        plan.invalidate_entities
            .contains(&EntityKey::PageSlug("old-page".to_string()))
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
