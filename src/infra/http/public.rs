use std::sync::Arc;

use axum::{Router, middleware, routing::get};

use crate::{
    application::{
        chrome::ChromeService, feed::FeedService, page::PageService, sitemap::SitemapService,
        snapshot_preview::SnapshotPreviewService, syndication::SyndicationService,
    },
    cache::{CacheState, response_cache_layer},
    infra::{db::PostgresRepositories, uploads::UploadStorage},
};

use super::{
    RouterState,
    middleware::{log_responses, set_request_context},
};

#[path = "public/assets.rs"]
mod assets;
#[path = "public/feed.rs"]
mod feed;
#[path = "public/meta.rs"]
mod meta;
#[path = "public/pages.rs"]
mod pages;
#[path = "public/previews.rs"]
mod previews;
#[path = "public/syndication.rs"]
mod syndication;

use assets::{favicon, public_health, serve_upload};
use feed::{index, month_index, post_detail, posts_partial, tag_index};
use pages::fallback_router;
use previews::{page_preview, page_snapshot_preview, post_preview, post_snapshot_preview};
use syndication::{atom_feed, robots_txt, rss_feed, sitemap};

#[derive(Clone)]
pub struct HttpState {
    pub feed: Arc<FeedService>,
    pub pages: Arc<PageService>,
    pub chrome: Arc<ChromeService>,
    pub syndication: Arc<SyndicationService>,
    pub sitemap: Arc<SitemapService>,
    pub db: Arc<PostgresRepositories>,
    pub upload_storage: Arc<UploadStorage>,
    pub snapshot_preview: Arc<SnapshotPreviewService>,
    pub cache: Option<CacheState>,
}

pub fn build_router(state: RouterState) -> Router<RouterState> {
    // Routes that should be cached (public content)
    // Middleware skips datastar-request headers, so streaming requests are not cached
    let cached_routes = Router::new()
        .route("/", get(index))
        .route("/tags/{tag}", get(tag_index))
        .route("/months/{month}", get(month_index))
        .route("/posts/{slug}", get(post_detail))
        .route("/ui/posts", get(posts_partial))
        .route("/sitemap.xml", get(sitemap))
        .route("/rss.xml", get(rss_feed))
        .route("/atom.xml", get(atom_feed))
        .route("/favicon.ico", get(favicon))
        .fallback(fallback_router);

    // Apply L1 cache layer conditionally
    let cached_routes = if let Some(cache_state) = state.http.cache.clone() {
        cached_routes.layer(middleware::from_fn_with_state(
            cache_state,
            response_cache_layer,
        ))
    } else {
        cached_routes
    };

    // Routes that should NOT be cached (previews, health, static assets)
    let static_routes = Router::new()
        .route("/posts/_preview/{id}", get(post_preview))
        .route("/pages/_preview/{id}", get(page_preview))
        .route("/posts/_preview/snapshot/{id}", get(post_snapshot_preview))
        .route("/pages/_preview/snapshot/{id}", get(page_snapshot_preview))
        .route("/_health/db", get(public_health))
        .route("/robots.txt", get(robots_txt))
        .route("/uploads/{*path}", get(serve_upload))
        .route(
            "/static/public/{*path}",
            get(crate::infra::assets::serve_public),
        )
        .route(
            "/static/common/{*path}",
            get(crate::infra::assets::serve_common),
        );

    cached_routes
        .merge(static_routes)
        .with_state(state)
        .layer(middleware::from_fn(log_responses))
        .layer(middleware::from_fn(set_request_context))
}
