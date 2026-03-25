use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

use axum::{
    Router,
    body::Body,
    http::{Method, Request, StatusCode},
    middleware,
    response::Response,
    routing::get,
};
use tower::ServiceExt;

use super::*;

#[test]
fn detect_format_html_default() {
    let req = Request::builder()
        .uri("/posts/hello")
        .body(Body::empty())
        .unwrap();
    assert_eq!(detect_format(&req), OutputFormat::Html);
}

#[test]
fn detect_format_rss() {
    let req = Request::builder()
        .uri("/rss.xml")
        .body(Body::empty())
        .unwrap();
    assert_eq!(detect_format(&req), OutputFormat::Rss);
}

#[test]
fn detect_format_atom() {
    let req = Request::builder()
        .uri("/atom.xml")
        .body(Body::empty())
        .unwrap();
    assert_eq!(detect_format(&req), OutputFormat::Atom);
}

#[test]
fn detect_format_sitemap() {
    let req = Request::builder()
        .uri("/sitemap.xml")
        .body(Body::empty())
        .unwrap();
    assert_eq!(detect_format(&req), OutputFormat::Sitemap);
}

#[test]
fn detect_format_json_from_accept_header() {
    let req = Request::builder()
        .uri("/posts/hello")
        .header("Accept", "application/json")
        .body(Body::empty())
        .unwrap();
    assert_eq!(detect_format(&req), OutputFormat::Json);
}

#[test]
fn detect_format_favicon() {
    let req = Request::builder()
        .uri("/favicon.ico")
        .body(Body::empty())
        .unwrap();
    assert_eq!(detect_format(&req), OutputFormat::Favicon);
}

fn build_cache_state(mut config: CacheConfig) -> (CacheState, Arc<L1Store>, Arc<CacheRegistry>) {
    config.enable_l1_cache = true;
    let l1 = Arc::new(L1Store::new(&config));
    let registry = Arc::new(CacheRegistry::new());
    let state = CacheState {
        config,
        l1: l1.clone(),
        registry: registry.clone(),
    };
    (state, l1, registry)
}

#[tokio::test]
async fn caches_tag_not_found_responses() {
    let config = CacheConfig {
        l1_response_body_limit_bytes: 1024,
        ..Default::default()
    };
    let (state, l1, _registry) = build_cache_state(config);

    let calls = Arc::new(AtomicUsize::new(0));
    let handler_calls = calls.clone();

    let app = Router::new()
        .route(
            "/tags/test",
            get(move || {
                let handler_calls = handler_calls.clone();
                async move {
                    handler_calls.fetch_add(1, Ordering::SeqCst);
                    crate::cache::deps::record(crate::cache::EntityKey::PostAggTags);
                    StatusCode::NOT_FOUND
                }
            }),
        )
        .layer(middleware::from_fn_with_state(state, response_cache_layer));

    let request = Request::builder()
        .uri("/tags/test")
        .method(Method::GET)
        .body(Body::empty())
        .unwrap();
    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert_eq!(l1.len(), 1);

    let request = Request::builder()
        .uri("/tags/test")
        .method(Method::GET)
        .body(Body::empty())
        .unwrap();
    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn skips_caching_when_body_exceeds_limit() {
    let config = CacheConfig {
        l1_response_body_limit_bytes: 4,
        ..Default::default()
    };
    let (state, l1, _registry) = build_cache_state(config);

    let calls = Arc::new(AtomicUsize::new(0));
    let handler_calls = calls.clone();

    let app = Router::new()
        .route(
            "/big",
            get(move || {
                let handler_calls = handler_calls.clone();
                async move {
                    handler_calls.fetch_add(1, Ordering::SeqCst);
                    Response::builder()
                        .status(StatusCode::OK)
                        .body(Body::from("12345"))
                        .unwrap()
                }
            }),
        )
        .layer(middleware::from_fn_with_state(state, response_cache_layer));

    let request = Request::builder()
        .uri("/big")
        .method(Method::GET)
        .body(Body::empty())
        .unwrap();
    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(l1.len(), 0);

    let request = Request::builder()
        .uri("/big")
        .method(Method::GET)
        .body(Body::empty())
        .unwrap();
    let _response = app.oneshot(request).await.unwrap();
    assert_eq!(calls.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn evicted_entries_unregister_registry() {
    let config = CacheConfig {
        l1_response_limit: 1,
        l1_response_body_limit_bytes: 1024,
        ..Default::default()
    };
    let (state, l1, registry) = build_cache_state(config);

    let app = Router::new()
        .route(
            "/a",
            get(|| async {
                crate::cache::deps::record(crate::cache::EntityKey::PostsIndex);
                Response::builder()
                    .status(StatusCode::OK)
                    .body(Body::from("a"))
                    .unwrap()
            }),
        )
        .route(
            "/b",
            get(|| async {
                crate::cache::deps::record(crate::cache::EntityKey::SiteSettings);
                Response::builder()
                    .status(StatusCode::OK)
                    .body(Body::from("b"))
                    .unwrap()
            }),
        )
        .layer(middleware::from_fn_with_state(state, response_cache_layer));

    let request = Request::builder()
        .uri("/a")
        .method(Method::GET)
        .body(Body::empty())
        .unwrap();
    let _response = app.clone().oneshot(request).await.unwrap();

    let request = Request::builder()
        .uri("/b")
        .method(Method::GET)
        .body(Body::empty())
        .unwrap();
    let _response = app.oneshot(request).await.unwrap();

    assert_eq!(l1.len(), 1);
    assert_eq!(registry.key_count(), 1);
    assert!(
        registry
            .keys_for_entity(&crate::cache::EntityKey::PostsIndex)
            .is_empty()
    );
    assert_eq!(
        registry
            .keys_for_entity(&crate::cache::EntityKey::SiteSettings)
            .len(),
        1
    );
}
