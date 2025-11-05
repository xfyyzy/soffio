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
use tracing::error;
use uuid::Uuid;

use crate::{
    application::repos::SettingsRepo,
    application::{
        chrome::ChromeService,
        error::HttpError,
        feed::{self, FeedError, FeedFilter, FeedService},
        page::PageService,
    },
    infra::{
        cache::ResponseCache,
        db::PostgresRepositories,
        uploads::{UploadStorage, UploadStorageError},
    },
    presentation::views::{
        IndexTemplate, LayoutChrome, LayoutContext, PageTemplate, PostTemplate, PostsPartial,
        render_not_found_response, render_template_response,
    },
};

use super::{
    DATASTAR_REQUEST_HEADER, db_health_response,
    middleware::{cache_public_responses, log_responses},
};

#[derive(Clone)]
pub struct HttpState {
    pub feed: Arc<FeedService>,
    pub pages: Arc<PageService>,
    pub chrome: Arc<ChromeService>,
    pub cache: Arc<ResponseCache>,
    pub db: Arc<PostgresRepositories>,
    pub upload_storage: Arc<UploadStorage>,
}

pub fn build_router(state: HttpState) -> Router {
    let cache = state.cache.clone();

    let cached_routes = Router::new()
        .route("/", get(index))
        .route("/tags/{tag}", get(tag_index))
        .route("/months/{month}", get(month_index))
        .route("/posts/{slug}", get(post_detail))
        .route("/ui/posts", get(posts_partial))
        .route("/_health/db", get(public_health))
        .fallback(fallback_router)
        .layer(middleware::from_fn_with_state(
            cache,
            cache_public_responses,
        ));

    let static_routes = Router::new()
        .route("/posts/_preview/{id}", get(post_preview))
        .route("/pages/_preview/{id}", get(page_preview))
        .route("/uploads/{*path}", get(serve_upload))
        .route("/favicon.ico", get(favicon))
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
            let view = LayoutContext::new(chrome.clone(), content);
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
            let view = LayoutContext::new(chrome.clone(), content);
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
            let view = LayoutContext::new(chrome.clone(), content);
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
            let view = LayoutContext::new(chrome.clone(), content);
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
            let view = LayoutContext::new(chrome.clone(), content);
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
            let view = LayoutContext::new(chrome.clone(), content);
            render_template_response(PageTemplate { view }, StatusCode::OK)
        }
        Ok(None) => render_not_found_response(chrome),
        Err(err) => err.into_response(),
    }
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
            let view = LayoutContext::new(chrome.clone(), page_view);
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
            StatusCode::SERVICE_UNAVAILABLE.into_response()
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
