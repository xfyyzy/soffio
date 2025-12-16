use std::{io::ErrorKind, sync::Arc};

use axum::{
    Router,
    body::Body,
    extract::{Path, Query, State},
    http::{
        HeaderMap, HeaderValue, Request, StatusCode,
        header::{CACHE_CONTROL, CONTENT_LENGTH, CONTENT_TYPE},
    },
    middleware,
    response::{IntoResponse, Response},
    routing::get,
};
use bytes::Bytes;
use serde::Deserialize;
use time::format_description::well_known::{Rfc2822, Rfc3339};
use tracing::error;
use uuid::Uuid;

use crate::{
    application::{
        chrome::ChromeService,
        error::HttpError,
        feed::{self, FeedError, FeedFilter, FeedService},
        page::PageService,
        snapshot_preview::SnapshotPreviewService,
    },
    cache::{CacheState, response_cache_layer},
    infra::{
        db::PostgresRepositories,
        uploads::{UploadStorage, UploadStorageError},
    },
    presentation::views::{
        IndexTemplate, LayoutChrome, LayoutContext, PageMetaView, PageTemplate, PageView,
        PostDetailContext, PostTemplate, PostsPartial, render_not_found_response,
        render_template_response,
    },
};

use super::{
    DATASTAR_REQUEST_HEADER, RouterState, db_health_response,
    middleware::{log_responses, set_request_context},
};
use crate::application::pagination::{PageCursor, PageRequest, PostCursor};
use crate::application::repos::{
    PageQueryFilter, PagesRepo, PostListScope, PostQueryFilter, PostsRepo, SettingsRepo,
};
use crate::domain::types::{PageStatus, PostStatus};

#[derive(Clone)]
pub struct HttpState {
    pub feed: Arc<FeedService>,
    pub pages: Arc<PageService>,
    pub chrome: Arc<ChromeService>,
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

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct CursorQuery {
    cursor: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct PartialQuery {
    cursor: Option<String>,
    tag: Option<String>,
    month: Option<String>,
}

async fn index(State(state): State<HttpState>, Query(query): Query<CursorQuery>) -> Response {
    let chrome = match state.chrome.load().await {
        Ok(chrome) => chrome,
        Err(err) => return err.into_response(),
    };

    match state
        .feed
        .page_context(FeedFilter::All, query.cursor.as_deref())
        .await
    {
        Ok(content) => {
            let canonical = canonical_url(&chrome.meta.canonical, "/");
            let view = LayoutContext::new(chrome.clone().with_canonical(canonical), content);
            render_template_response(IndexTemplate { view }, StatusCode::OK)
        }
        Err(err) => feed_error_to_response(err, chrome),
    }
}

async fn tag_index(
    State(state): State<HttpState>,
    Path(tag): Path<String>,
    Query(query): Query<CursorQuery>,
) -> Response {
    let chrome = match state.chrome.load().await {
        Ok(chrome) => chrome,
        Err(err) => return err.into_response(),
    };

    match state.feed.is_known_tag(&tag).await {
        Ok(true) => {
            let content = match state
                .feed
                .page_context(FeedFilter::Tag(tag.clone()), query.cursor.as_deref())
                .await
            {
                Ok(content) => content,
                Err(err) => return feed_error_to_response(err, chrome),
            };
            let canonical = canonical_url(&chrome.meta.canonical, &format!("/tags/{tag}"));
            let view = LayoutContext::new(chrome.clone().with_canonical(canonical), content);
            render_template_response(IndexTemplate { view }, StatusCode::OK)
        }
        Ok(false) => render_not_found_response(chrome),
        Err(err) => feed_error_to_response(err, chrome),
    }
}

async fn month_index(
    State(state): State<HttpState>,
    Path(month): Path<String>,
    Query(query): Query<CursorQuery>,
) -> Response {
    let chrome = match state.chrome.load().await {
        Ok(chrome) => chrome,
        Err(err) => return err.into_response(),
    };

    match state.feed.is_known_month(&month).await {
        Ok(true) => {
            let content = match state
                .feed
                .page_context(FeedFilter::Month(month.clone()), query.cursor.as_deref())
                .await
            {
                Ok(content) => content,
                Err(err) => return feed_error_to_response(err, chrome),
            };
            let canonical = canonical_url(&chrome.meta.canonical, &format!("/months/{month}"));
            let view = LayoutContext::new(chrome.clone().with_canonical(canonical), content);
            render_template_response(IndexTemplate { view }, StatusCode::OK)
        }
        Ok(false) => render_not_found_response(chrome),
        Err(err) => feed_error_to_response(err, chrome),
    }
}

async fn serve_upload(State(state): State<HttpState>, Path(path): Path<String>) -> Response {
    const SOURCE: &str = "infra::http::public::serve_upload";

    match state.upload_storage.read(&path).await {
        Ok(bytes) => build_upload_response(&path, bytes),
        Err(UploadStorageError::InvalidPath) => HttpError::new(
            SOURCE,
            StatusCode::NOT_FOUND,
            "Upload not found",
            "The requested upload is not available",
        )
        .into_response(),
        Err(UploadStorageError::Io(err)) if err.kind() == ErrorKind::NotFound => HttpError::new(
            SOURCE,
            StatusCode::NOT_FOUND,
            "Upload not found",
            "The requested upload is not available",
        )
        .into_response(),
        Err(err) => {
            error!(
                target = SOURCE,
                path = %path,
                error = %err,
                "failed to read stored upload"
            );
            HttpError::new(
                SOURCE,
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to read uploaded file",
                err.to_string(),
            )
            .into_response()
        }
    }
}

async fn posts_partial(
    State(state): State<HttpState>,
    headers: HeaderMap,
    Query(params): Query<PartialQuery>,
) -> Result<Response, HttpError> {
    if params.tag.is_some() && params.month.is_some() {
        return Err(HttpError::new(
            "infra::http::posts_partial",
            StatusCode::BAD_REQUEST,
            "Conflicting filters",
            "Received both tag and month filters",
        ));
    }

    let filter = if let Some(tag) = params.tag.clone() {
        match state.feed.is_known_tag(&tag).await {
            Ok(true) => FeedFilter::Tag(tag),
            Ok(false) => {
                return Err(HttpError::new(
                    "infra::http::posts_partial",
                    StatusCode::NOT_FOUND,
                    "Unknown tag",
                    "Requested tag could not be found",
                ));
            }
            Err(err) => return Err(err.into()),
        }
    } else if let Some(month) = params.month.clone() {
        match state.feed.is_known_month(&month).await {
            Ok(true) => FeedFilter::Month(month),
            Ok(false) => {
                return Err(HttpError::new(
                    "infra::http::posts_partial",
                    StatusCode::NOT_FOUND,
                    "Unknown month",
                    "Requested month archive could not be found",
                ));
            }
            Err(err) => return Err(err.into()),
        }
    } else {
        FeedFilter::All
    };

    let is_datastar = headers.contains_key(DATASTAR_REQUEST_HEADER);
    let cursor = params.cursor.as_deref();
    let load_more_query = filter.load_more_query();

    if is_datastar {
        let payload = state.feed.append_payload(filter.clone(), cursor).await?;
        let response = feed::build_datastar_append_response(payload, load_more_query)?;
        return Ok(response);
    }

    let content = state.feed.page_context(filter, cursor).await?;

    Ok(render_template_response(
        PostsPartial { content },
        StatusCode::OK,
    ))
}

async fn post_detail(State(state): State<HttpState>, Path(slug): Path<String>) -> Response {
    let chrome = match state.chrome.load().await {
        Ok(chrome) => chrome,
        Err(err) => return err.into_response(),
    };

    match state.feed.post_detail(&slug).await {
        Ok(Some(content)) => {
            let canonical = canonical_url(&chrome.meta.canonical, &format!("/posts/{slug}"));
            let meta = post_meta(&chrome, &content, canonical);
            let view = LayoutContext::new(chrome.clone().with_meta(meta), content);
            render_template_response(PostTemplate { view }, StatusCode::OK)
        }
        Ok(None) => render_not_found_response(chrome),
        Err(err) => feed_error_to_response(err, chrome),
    }
}

async fn post_preview(State(state): State<HttpState>, Path(id): Path<Uuid>) -> Response {
    let chrome = match state.chrome.load().await {
        Ok(chrome) => chrome,
        Err(err) => return err.into_response(),
    };

    match state.feed.post_preview(id).await {
        Ok(Some(content)) => {
            let canonical = canonical_url(&chrome.meta.canonical, &format!("/posts/_preview/{id}"));
            let meta = post_meta(&chrome, &content, canonical);
            let view = LayoutContext::new(chrome.clone().with_meta(meta), content);
            render_template_response(PostTemplate { view }, StatusCode::OK)
        }
        Ok(None) => render_not_found_response(chrome),
        Err(err) => feed_error_to_response(err, chrome),
    }
}

async fn page_preview(State(state): State<HttpState>, Path(id): Path<Uuid>) -> Response {
    let chrome = match state.chrome.load().await {
        Ok(chrome) => chrome,
        Err(err) => return err.into_response(),
    };

    match state.pages.page_preview(id).await {
        Ok(Some(content)) => {
            let canonical = canonical_url(&chrome.meta.canonical, &format!("/pages/_preview/{id}"));
            let meta = page_meta(&chrome, &content, canonical);
            let view = LayoutContext::new(chrome.clone().with_meta(meta), content);
            render_template_response(PageTemplate { view }, StatusCode::OK)
        }
        Ok(None) => render_not_found_response(chrome),
        Err(err) => err.into_response(),
    }
}

async fn post_snapshot_preview(State(state): State<HttpState>, Path(id): Path<Uuid>) -> Response {
    let chrome = match state.chrome.load().await {
        Ok(chrome) => chrome,
        Err(err) => return err.into_response(),
    };

    match state.snapshot_preview.post_snapshot_view(id).await {
        Ok(Some(content)) => {
            let canonical = canonical_url(
                &chrome.meta.canonical,
                &format!("/posts/_preview/snapshot/{id}"),
            );
            let meta = post_meta(&chrome, &content, canonical);
            let mut response = render_template_response(
                PostTemplate {
                    view: LayoutContext::new(chrome.clone().with_meta(meta), content),
                },
                StatusCode::OK,
            );
            set_no_store(&mut response);
            response
        }
        Ok(None) => render_not_found_response(chrome),
        Err(err) => err.into_response(),
    }
}

async fn page_snapshot_preview(State(state): State<HttpState>, Path(id): Path<Uuid>) -> Response {
    let chrome = match state.chrome.load().await {
        Ok(chrome) => chrome,
        Err(err) => return err.into_response(),
    };

    match state.snapshot_preview.page_snapshot_view(id).await {
        Ok(Some(content)) => {
            let canonical = canonical_url(
                &chrome.meta.canonical,
                &format!("/pages/_preview/snapshot/{id}"),
            );
            let meta = page_meta(&chrome, &content, canonical);
            let mut response = render_template_response(
                PageTemplate {
                    view: LayoutContext::new(chrome.clone().with_meta(meta), content),
                },
                StatusCode::OK,
            );
            set_no_store(&mut response);
            response
        }
        Ok(None) => render_not_found_response(chrome),
        Err(err) => err.into_response(),
    }
}

fn set_no_store(response: &mut Response) {
    let value = HeaderValue::from_static("no-store");
    response.headers_mut().insert(CACHE_CONTROL, value);
}

async fn fallback_router(State(state): State<HttpState>, request: Request<Body>) -> Response {
    let raw_path = request.uri().path().trim_matches('/');
    let slug = raw_path.trim_end_matches('/');

    let chrome = match state.chrome.load().await {
        Ok(chrome) => chrome,
        Err(err) => return err.into_response(),
    };

    if slug.is_empty() {
        return render_not_found_response(chrome);
    }

    match state.pages.page_view(slug).await {
        Ok(Some(page_view)) => {
            let canonical = canonical_url(&chrome.meta.canonical, &format!("/{slug}"));
            let meta = page_meta(&chrome, &page_view, canonical);
            let view = LayoutContext::new(chrome.clone().with_meta(meta), page_view);
            render_template_response(PageTemplate { view }, StatusCode::OK)
        }
        Ok(None) => render_not_found_response(chrome),
        Err(err) => err.into_response(),
    }
}

fn feed_error_to_response(err: FeedError, chrome: LayoutChrome) -> Response {
    match err {
        FeedError::UnknownTag => {
            let mut response = render_not_found_response(chrome);
            crate::application::error::ErrorReport::from_message(
                "infra::http::feed_error_to_response",
                StatusCode::NOT_FOUND,
                "Unknown tag",
            )
            .attach(&mut response);
            response
        }
        FeedError::UnknownMonth => {
            let mut response = render_not_found_response(chrome);
            crate::application::error::ErrorReport::from_message(
                "infra::http::feed_error_to_response",
                StatusCode::NOT_FOUND,
                "Unknown month",
            )
            .attach(&mut response);
            response
        }
        err => HttpError::from(err).into_response(),
    }
}

async fn public_health(State(state): State<HttpState>) -> Response {
    db_health_response(state.db.health_check().await)
}

async fn favicon(State(state): State<HttpState>) -> Response {
    match state.db.load_site_settings().await {
        Ok(settings) => Response::builder()
            .status(StatusCode::OK)
            .header(CONTENT_TYPE, "image/svg+xml; charset=utf-8")
            .header(CACHE_CONTROL, "public, max-age=3600")
            .body(Body::from(settings.favicon_svg))
            .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response()),
        Err(err) => {
            error!(target = "soffio::http::favicon", error = %err, "failed to load favicon from settings");
            let mut response = StatusCode::SERVICE_UNAVAILABLE.into_response();
            crate::application::error::ErrorReport::from_error(
                "infra::http::public::favicon",
                StatusCode::SERVICE_UNAVAILABLE,
                &err,
            )
            .attach(&mut response);
            response
        }
    }
}

fn build_upload_response(path: &str, bytes: Bytes) -> Response {
    let mut response = Response::new(Body::from(bytes.clone()));
    *response.status_mut() = StatusCode::OK;

    let headers = response.headers_mut();
    let mime = mime_guess::from_path(path).first_or_octet_stream();
    if let Ok(value) = HeaderValue::from_str(mime.as_ref()) {
        headers.insert(CONTENT_TYPE, value);
    }
    if let Ok(value) = HeaderValue::from_str(&bytes.len().to_string()) {
        headers.insert(CONTENT_LENGTH, value);
    }
    headers.insert(
        CACHE_CONTROL,
        HeaderValue::from_static("public, max-age=31536000, immutable"),
    );

    response
}

async fn sitemap(State(state): State<HttpState>) -> Response {
    match build_sitemap_xml(&state).await {
        Ok(body) => xml_response(body, "application/xml"),
        Err(err) => err.into_response(),
    }
}

async fn rss_feed(State(state): State<HttpState>) -> Response {
    match build_rss_xml(&state).await {
        Ok(body) => xml_response(body, "application/rss+xml"),
        Err(err) => err.into_response(),
    }
}

async fn atom_feed(State(state): State<HttpState>) -> Response {
    match build_atom_xml(&state).await {
        Ok(body) => xml_response(body, "application/atom+xml"),
        Err(err) => err.into_response(),
    }
}

async fn robots_txt(State(state): State<HttpState>) -> Response {
    let settings = match state.db.load_site_settings().await {
        Ok(s) => s,
        Err(err) => {
            return HttpError::new(
                "infra::http::public::robots",
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to load settings",
                err.to_string(),
            )
            .into_response();
        }
    };

    let base = normalize_public_site_url(&settings.public_site_url);
    let sitemap_url = format!("{base}sitemap.xml");
    let body = format!("User-agent: *\nAllow: /\nSitemap: {sitemap_url}\n");

    plain_response(body)
}

pub(crate) fn post_meta(
    chrome: &LayoutChrome,
    content: &PostDetailContext,
    canonical: String,
) -> PageMetaView {
    let description = fallback_description(&content.excerpt, &chrome.meta.description);

    chrome
        .meta
        .clone()
        .with_canonical(canonical)
        .with_content(content.title.clone(), description)
}

pub(crate) fn page_meta(chrome: &LayoutChrome, page: &PageView, canonical: String) -> PageMetaView {
    let derived = summarize_html(&page.content_html, 180);
    let description = fallback_description(&derived, &chrome.meta.description);

    chrome
        .meta
        .clone()
        .with_canonical(canonical)
        .with_content(page.title.clone(), description)
}

fn fallback_description(candidate: &str, fallback: &str) -> String {
    let trimmed = candidate.trim();
    if trimmed.is_empty() {
        fallback.to_string()
    } else {
        trimmed.to_string()
    }
}

fn summarize_html(html: &str, max_len: usize) -> String {
    let mut text = String::with_capacity(max_len);
    let mut in_tag = false;
    let mut last_was_space = false;

    for ch in html.chars() {
        match ch {
            '<' => {
                in_tag = true;
                continue;
            }
            '>' => {
                in_tag = false;
                last_was_space = false;
                continue;
            }
            _ if in_tag => continue,
            c if c.is_whitespace() => {
                if !last_was_space && !text.is_empty() {
                    text.push(' ');
                }
                last_was_space = true;
            }
            c => {
                text.push(c);
                last_was_space = false;
            }
        }

        if text.len() >= max_len {
            break;
        }
    }

    text.trim().to_string()
}

pub(crate) fn canonical_url(base: &str, path: &str) -> String {
    let root = normalize_public_site_url(base);
    let trimmed = path.trim_start_matches('/');
    if trimmed.is_empty() {
        root.clone()
    } else {
        format!("{root}{trimmed}")
    }
}

fn normalize_public_site_url(url: &str) -> String {
    let trimmed = url.trim_end_matches('/');
    format!("{trimmed}/")
}

fn xml_response(body: String, content_type: &str) -> Response {
    Response::builder()
        .status(StatusCode::OK)
        .header(CONTENT_TYPE, content_type)
        .body(Body::from(body))
        .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())
}

fn plain_response(body: String) -> Response {
    Response::builder()
        .status(StatusCode::OK)
        .header(CONTENT_TYPE, "text/plain; charset=utf-8")
        .body(Body::from(body))
        .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())
}

async fn build_sitemap_xml(state: &HttpState) -> Result<String, HttpError> {
    const SOURCE: &str = "infra::http::public::sitemap";

    // Record dependencies for L1 cache invalidation
    // Note: Individual page slugs are recorded as they're iterated below
    crate::cache::deps::record(crate::cache::EntityKey::Sitemap);
    crate::cache::deps::record(crate::cache::EntityKey::SiteSettings);
    crate::cache::deps::record(crate::cache::EntityKey::PostsIndex);

    let settings = state.db.load_site_settings().await.map_err(|err| {
        HttpError::new(
            SOURCE,
            StatusCode::INTERNAL_SERVER_ERROR,
            "Failed to load settings",
            err.to_string(),
        )
    })?;

    let base = normalize_public_site_url(&settings.public_site_url);
    let posts_repo: Arc<dyn PostsRepo> = state.db.clone();
    let pages_repo: Arc<dyn PagesRepo> = state.db.clone();

    let mut entries = Vec::new();

    entries.push(sitemap_entry(&base, "/", Some(settings.updated_at)));

    // Posts
    let mut post_cursor: Option<PostCursor> = None;
    loop {
        let page = posts_repo
            .list_posts(
                PostListScope::Public,
                &PostQueryFilter::default(),
                PageRequest::new(200, post_cursor),
            )
            .await
            .map_err(|err| {
                HttpError::new(
                    SOURCE,
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Failed to list posts",
                    err.to_string(),
                )
            })?;

        for post in page.items.into_iter() {
            if post.status != PostStatus::Published {
                continue;
            }
            let lastmod = post.published_at.unwrap_or(post.updated_at);
            entries.push(sitemap_entry(
                &base,
                &format!("/posts/{}", post.slug),
                Some(lastmod),
            ));
        }

        post_cursor = match page.next_cursor {
            Some(next) => Some(PostCursor::decode(&next).map_err(|err| {
                HttpError::new(
                    SOURCE,
                    StatusCode::BAD_REQUEST,
                    "Failed to decode post cursor",
                    err.to_string(),
                )
            })?),
            None => break,
        };
    }

    // Pages
    let mut page_cursor: Option<PageCursor> = None;
    loop {
        let page = pages_repo
            .list_pages(
                Some(PageStatus::Published),
                200,
                page_cursor,
                &PageQueryFilter::default(),
            )
            .await
            .map_err(|err| {
                HttpError::new(
                    SOURCE,
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Failed to list pages",
                    err.to_string(),
                )
            })?;

        for record in page.items.into_iter() {
            if record.published_at.is_none() {
                continue;
            }
            let lastmod = record.published_at.unwrap_or(record.updated_at);
            entries.push(sitemap_entry(
                &base,
                &format!("/{}", record.slug),
                Some(lastmod),
            ));
        }

        page_cursor = match page.next_cursor {
            Some(next) => Some(PageCursor::decode(&next).map_err(|err| {
                HttpError::new(
                    SOURCE,
                    StatusCode::BAD_REQUEST,
                    "Failed to decode page cursor",
                    err.to_string(),
                )
            })?),
            None => break,
        };
    }

    let mut xml = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">\n",
    );
    for entry in entries {
        xml.push_str(&entry);
    }
    xml.push_str("</urlset>\n");
    Ok(xml)
}

fn sitemap_entry(base: &str, path: &str, lastmod: Option<time::OffsetDateTime>) -> String {
    let loc = canonical_url(base, path);
    let lastmod_str = lastmod
        .and_then(|dt| dt.format(&Rfc3339).ok())
        .unwrap_or_default();
    if lastmod_str.is_empty() {
        format!("  <url><loc>{loc}</loc></url>\n")
    } else {
        format!("  <url><loc>{loc}</loc><lastmod>{lastmod_str}</lastmod></url>\n")
    }
}

async fn build_rss_xml(state: &HttpState) -> Result<String, HttpError> {
    const SOURCE: &str = "infra::http::public::rss";

    // Record dependencies for L1 cache invalidation
    crate::cache::deps::record(crate::cache::EntityKey::Feed);
    crate::cache::deps::record(crate::cache::EntityKey::SiteSettings);
    crate::cache::deps::record(crate::cache::EntityKey::PostsIndex);

    let settings = state.db.load_site_settings().await.map_err(|err| {
        HttpError::new(
            SOURCE,
            StatusCode::INTERNAL_SERVER_ERROR,
            "Failed to load settings",
            err.to_string(),
        )
    })?;
    let base = normalize_public_site_url(&settings.public_site_url);

    let posts_repo: Arc<dyn PostsRepo> = state.db.clone();
    let page = posts_repo
        .list_posts(
            PostListScope::Public,
            &PostQueryFilter::default(),
            PageRequest::new(100, None),
        )
        .await
        .map_err(|err| {
            HttpError::new(
                SOURCE,
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to list posts",
                err.to_string(),
            )
        })?;

    let mut items = String::new();
    for post in page
        .items
        .into_iter()
        .filter(|p| p.status == PostStatus::Published)
    {
        let published = post.published_at.unwrap_or(post.updated_at);
        let pub_date = published
            .format(&Rfc2822)
            .unwrap_or_else(|_| published.to_string());
        let link = format!("{base}posts/{}", post.slug);
        items.push_str(&format!(
            "    <item>\n      <title>{}</title>\n      <link>{}</link>\n      <guid>{}</guid>\n      <pubDate>{}</pubDate>\n      <description><![CDATA[{}]]></description>\n    </item>\n",
            xml_escape(&post.title),
            link,
            link,
            pub_date,
            xml_escape(&post.excerpt),
        ));
    }

    let channel = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<rss version=\"2.0\">\n  <channel>\n    <title>{}</title>\n    <link>{}</link>\n    <description>{}</description>\n{}  </channel>\n</rss>\n",
        xml_escape(&settings.meta_title),
        base,
        xml_escape(&settings.meta_description),
        items
    );

    Ok(channel)
}

async fn build_atom_xml(state: &HttpState) -> Result<String, HttpError> {
    const SOURCE: &str = "infra::http::public::atom";

    let settings = state.db.load_site_settings().await.map_err(|err| {
        HttpError::new(
            SOURCE,
            StatusCode::INTERNAL_SERVER_ERROR,
            "Failed to load settings",
            err.to_string(),
        )
    })?;
    let base = normalize_public_site_url(&settings.public_site_url);

    let posts_repo: Arc<dyn PostsRepo> = state.db.clone();
    let page = posts_repo
        .list_posts(
            PostListScope::Public,
            &PostQueryFilter::default(),
            PageRequest::new(100, None),
        )
        .await
        .map_err(|err| {
            HttpError::new(
                SOURCE,
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to list posts",
                err.to_string(),
            )
        })?;

    let updated = settings
        .updated_at
        .format(&Rfc3339)
        .unwrap_or_else(|_| settings.updated_at.to_string());

    let mut entries = String::new();
    for post in page
        .items
        .into_iter()
        .filter(|p| p.status == PostStatus::Published)
    {
        let published = post.published_at.unwrap_or(post.updated_at);
        let published_str = published
            .format(&Rfc3339)
            .unwrap_or_else(|_| published.to_string());
        let link = format!("{base}posts/{}", post.slug);
        entries.push_str(&format!(
            "  <entry>\n    <title>{}</title>\n    <link href=\"{}\"/>\n    <id>{}</id>\n    <updated>{}</updated>\n    <summary><![CDATA[{}]]></summary>\n  </entry>\n",
            xml_escape(&post.title),
            link,
            link,
            published_str,
            xml_escape(&post.excerpt),
        ));
    }

    let feed = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<feed xmlns=\"http://www.w3.org/2005/Atom\">\n  <title>{}</title>\n  <id>{}</id>\n  <updated>{}</updated>\n  <link href=\"{}atom.xml\" rel=\"self\"/>\n{}\n</feed>\n",
        xml_escape(&settings.meta_title),
        base,
        updated,
        base,
        entries
    );

    Ok(feed)
}

fn xml_escape(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}
