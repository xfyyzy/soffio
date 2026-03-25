use axum::{
    extract::{Path, State},
    http::{HeaderValue, StatusCode, header::CACHE_CONTROL},
    response::{IntoResponse, Response},
};
use uuid::Uuid;

use crate::presentation::views::{
    LayoutContext, PageTemplate, PostTemplate, render_not_found_response, render_template_response,
};

use super::{
    HttpState,
    feed::feed_error_to_response,
    meta::{canonical_url, page_meta, post_meta},
};

pub(super) async fn post_preview(State(state): State<HttpState>, Path(id): Path<Uuid>) -> Response {
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

pub(super) async fn page_preview(State(state): State<HttpState>, Path(id): Path<Uuid>) -> Response {
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

pub(super) async fn post_snapshot_preview(
    State(state): State<HttpState>,
    Path(id): Path<Uuid>,
) -> Response {
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

pub(super) async fn page_snapshot_preview(
    State(state): State<HttpState>,
    Path(id): Path<Uuid>,
) -> Response {
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
