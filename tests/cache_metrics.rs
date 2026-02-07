use std::collections::HashSet;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use axum::{
    Router,
    body::Body,
    extract::Path,
    http::{Method, Request, StatusCode},
    middleware,
    routing::get,
};
use metrics_util::debugging::DebuggingRecorder;
use soffio::cache::{
    CacheConfig, CacheConsumer, CacheRegistry, CacheState, EventKind, EventQueue, L0Store, L1Store,
    response_cache_layer,
};
use soffio::domain::entities::PostRecord;
use soffio::domain::types::PostStatus;
use soffio::infra::db::PostgresRepositories;
use sqlx::PgPool;
use time::OffsetDateTime;
use tower::ServiceExt;
use uuid::Uuid;

fn sample_post(id: Uuid, slug: &str) -> PostRecord {
    PostRecord {
        id,
        slug: slug.to_string(),
        title: "Metrics Test Post".to_string(),
        excerpt: "".to_string(),
        body_markdown: "".to_string(),
        status: PostStatus::Published,
        pinned: false,
        scheduled_at: None,
        published_at: Some(OffsetDateTime::now_utc()),
        archived_at: None,
        summary_markdown: None,
        summary_html: None,
        created_at: OffsetDateTime::now_utc(),
        updated_at: OffsetDateTime::now_utc(),
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn cache_paths_emit_expected_metric_keys(pool: PgPool) {
    let recorder = DebuggingRecorder::new();
    let snapshotter = recorder.snapshotter();
    recorder
        .install()
        .expect("debug metrics recorder should install in this test process");

    // L0 cache hit/miss/evict
    let l0_config = CacheConfig {
        l0_post_limit: 1,
        ..Default::default()
    };
    let l0 = L0Store::new(&l0_config);
    let first_post_id = Uuid::new_v4();
    let second_post_id = Uuid::new_v4();

    assert!(l0.get_post_by_id(first_post_id).is_none());
    l0.set_post(sample_post(first_post_id, "metrics-post-1"));
    assert!(l0.get_post_by_id(first_post_id).is_some());
    l0.set_post(sample_post(second_post_id, "metrics-post-2"));

    // Event queue length + drop metrics
    let queue = Arc::new(EventQueue::new_with_limit(1));
    queue.publish(EventKind::SiteSettingsUpdated);
    queue.publish(EventKind::NavigationUpdated);
    let _ = queue.drain(1);

    // Consumer latencies (consume + warm)
    let consumer_config = CacheConfig::default();
    let consumer_l0 = Arc::new(L0Store::new(&consumer_config));
    let consumer_l1 = Arc::new(L1Store::new(&consumer_config));
    let registry = Arc::new(CacheRegistry::new());
    let consumer_queue = Arc::new(EventQueue::new_with_limit(16));
    let repos = Arc::new(PostgresRepositories::new(pool));
    let consumer = CacheConsumer::new(
        consumer_config,
        consumer_l0,
        consumer_l1,
        registry,
        consumer_queue.clone(),
        repos,
    );

    consumer_queue.publish(EventKind::WarmupOnStartup);
    assert!(consumer.consume_full().await);

    // L1 hit/miss + eviction metrics through middleware path
    let l1_config = CacheConfig {
        l1_response_limit: 1,
        l1_response_body_limit_bytes: 4096,
        ..Default::default()
    };
    let l1 = Arc::new(L1Store::new(&l1_config));
    let l1_registry = Arc::new(CacheRegistry::new());
    let cache_state = CacheState {
        config: l1_config,
        l1,
        registry: l1_registry,
    };

    let calls = Arc::new(AtomicUsize::new(0));
    let app = Router::new()
        .route(
            "/tags/{slug}",
            get(move |Path(_slug): Path<String>| {
                let calls = Arc::clone(&calls);
                async move {
                    calls.fetch_add(1, Ordering::SeqCst);
                    StatusCode::OK
                }
            }),
        )
        .layer(middleware::from_fn_with_state(
            cache_state,
            response_cache_layer,
        ));

    for uri in ["/tags/one", "/tags/one", "/tags/two"] {
        let request = Request::builder()
            .method(Method::GET)
            .uri(uri)
            .body(Body::empty())
            .expect("request should build");
        let response = app
            .clone()
            .oneshot(request)
            .await
            .expect("router should respond");
        assert_eq!(response.status(), StatusCode::OK);
    }

    let names: HashSet<String> = snapshotter
        .snapshot()
        .into_vec()
        .into_iter()
        .map(|(composite_key, _, _, _)| composite_key.key().name().to_string())
        .collect();

    let expected = [
        "soffio_cache_l0_hit_total",
        "soffio_cache_l0_miss_total",
        "soffio_cache_l0_evict_total",
        "soffio_cache_l1_hit_total",
        "soffio_cache_l1_miss_total",
        "soffio_cache_l1_evict_total",
        "soffio_cache_event_queue_len",
        "soffio_cache_event_dropped_total",
        "soffio_cache_consume_ms",
        "soffio_cache_warm_ms",
    ];

    for metric in expected {
        assert!(names.contains(metric), "missing metric: {metric}");
    }
}
