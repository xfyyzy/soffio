use axum::{
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    response::{IntoResponse, Response},
};

use crate::presentation::views::{
    LayoutContext, PageTemplate, render_not_found_response, render_template_response,
};

use super::{
    HttpState,
    meta::{canonical_url, page_meta},
};

pub(super) async fn fallback_router(
    State(state): State<HttpState>,
    request: Request<Body>,
) -> Response {
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
