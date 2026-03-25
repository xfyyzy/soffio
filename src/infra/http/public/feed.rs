use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use serde::Deserialize;

use crate::{
    application::{
        error::{ErrorReport, HttpError},
        feed::{self, FeedError, FeedFilter},
    },
    presentation::views::{
        IndexTemplate, LayoutChrome, LayoutContext, PostTemplate, PostsPartial,
        render_not_found_response, render_template_response,
    },
};

use super::{
    HttpState,
    meta::{canonical_url, post_meta},
};

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub(super) struct CursorQuery {
    cursor: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub(super) struct PartialQuery {
    cursor: Option<String>,
    tag: Option<String>,
    month: Option<String>,
}

pub(super) async fn index(
    State(state): State<HttpState>,
    Query(query): Query<CursorQuery>,
) -> Response {
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

pub(super) async fn tag_index(
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

pub(super) async fn month_index(
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

pub(super) async fn posts_partial(
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

    let is_datastar = headers.contains_key(super::super::DATASTAR_REQUEST_HEADER);
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

pub(super) async fn post_detail(
    State(state): State<HttpState>,
    Path(slug): Path<String>,
) -> Response {
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

pub(super) fn feed_error_to_response(err: FeedError, chrome: LayoutChrome) -> Response {
    match err {
        FeedError::UnknownTag => {
            let mut response = render_not_found_response(chrome);
            ErrorReport::from_message(
                "infra::http::feed_error_to_response",
                StatusCode::NOT_FOUND,
                "Unknown tag",
            )
            .attach(&mut response);
            response
        }
        FeedError::UnknownMonth => {
            let mut response = render_not_found_response(chrome);
            ErrorReport::from_message(
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
