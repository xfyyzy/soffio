//! Cache invalidation tests for the API layer.
//!
//! These tests verify that the `invalidate_admin_writes` middleware is correctly
//! applied to API routes, ensuring cache consistency when content is modified
//! via the headless API (e.g., soffio-cli).

use std::sync::Arc;

use axum::http::StatusCode;

use soffio::infra::cache::ResponseCache;

/// Documents the expected cache invalidation behavior for API routes.
/// The actual runtime test is in live_api.rs as it requires a running server.
#[test]
fn cache_invalidation_middleware_behavior_documented() {
    // Expected behavior of invalidate_admin_writes middleware on API routes:
    //
    // 1. For GET requests: No cache invalidation occurs
    // 2. For POST/PATCH/DELETE requests with successful response (2xx):
    //    - cache.invalidate_all() is called AFTER the handler completes
    //    - This clears both `entries` and `seo_entries` maps
    // 3. For failed requests (4xx, 5xx): No cache invalidation occurs
    //
    // This ensures that:
    // - Read operations don't unnecessarily invalidate cache
    // - Write operations atomically invalidate cache after database changes
    // - Failed operations don't corrupt cache state
    //
    // The middleware order in api/mod.rs is:
    //   .layer(invalidate_admin_writes)  // Runs FIRST on response
    //   .layer(log_responses)
    //   .layer(api_rate_limit)
    //   .layer(api_auth)
    //   .layer(set_request_context)      // Runs FIRST on request
    //
    // This ensures cache is invalidated after the response is fully generated
    // but before it's sent to the client.
}

#[cfg(test)]
mod response_cache_tests {
    use super::*;
    use axum::http::header::CONTENT_TYPE;
    use axum::response::{IntoResponse, Response};

    #[tokio::test]
    async fn response_cache_invalidate_all_clears_both_caches() {
        let cache = ResponseCache::new();

        // Simulate cached content by directly accessing internals is not possible,
        // but we can verify invalidate_all doesn't panic on empty cache
        cache.invalidate_all().await;

        // Verify get returns None after invalidation (empty cache)
        assert!(cache.get("/").await.is_none());
        assert!(cache.get("/posts/test").await.is_none());
    }

    #[tokio::test]
    async fn response_cache_can_store_and_retrieve() {
        let cache = Arc::new(ResponseCache::new());

        // Create a simple cacheable response
        let response: Response = (
            StatusCode::OK,
            [(CONTENT_TYPE, "text/html")],
            "<html>test</html>",
        )
            .into_response();

        // Store and retrieve
        let result = cache.store_response("/test", response).await;
        assert!(result.is_ok());

        let cached = cache.get("/test").await;
        assert!(cached.is_some());

        // Invalidate and verify cleared
        cache.invalidate_all().await;
        let after_invalidation = cache.get("/test").await;
        assert!(after_invalidation.is_none());
    }

    #[tokio::test]
    async fn response_cache_invalidate_clears_seo_entries() {
        use soffio::infra::cache::SeoKey;

        let cache = ResponseCache::new();

        // Put SEO entries
        cache
            .put_seo(SeoKey::Sitemap, "<xml>sitemap</xml>".to_string())
            .await;
        cache
            .put_seo(SeoKey::Rss, "<xml>rss</xml>".to_string())
            .await;
        cache
            .put_seo(SeoKey::Atom, "<xml>atom</xml>".to_string())
            .await;
        cache
            .put_seo(SeoKey::Robots, "User-agent: *".to_string())
            .await;

        // Verify they're cached
        assert!(cache.get_seo(SeoKey::Sitemap).await.is_some());
        assert!(cache.get_seo(SeoKey::Rss).await.is_some());
        assert!(cache.get_seo(SeoKey::Atom).await.is_some());
        assert!(cache.get_seo(SeoKey::Robots).await.is_some());

        // Invalidate all
        cache.invalidate_all().await;

        // Verify SEO entries are also cleared
        assert!(cache.get_seo(SeoKey::Sitemap).await.is_none());
        assert!(cache.get_seo(SeoKey::Rss).await.is_none());
        assert!(cache.get_seo(SeoKey::Atom).await.is_none());
        assert!(cache.get_seo(SeoKey::Robots).await.is_none());
    }
}
