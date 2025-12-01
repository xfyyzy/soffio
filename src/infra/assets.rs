//! Embedded static asset serving utilities.

use std::borrow::Cow;

use axum::{
    body::Body,
    extract::Path,
    http::{HeaderValue, StatusCode, header},
    response::{IntoResponse, Response},
};
use bytes::Bytes;
use include_dir::{Dir, include_dir};
use mime_guess::{Mime, MimeGuess};

use crate::application::error::ErrorReport;

static STATIC_PUBLIC_ASSETS: Dir<'_> = include_dir!("$OUT_DIR/static_public");
static STATIC_ADMIN_ASSETS: Dir<'_> = include_dir!("$OUT_DIR/static_admin");
static STATIC_SHARED_ASSETS: Dir<'_> = include_dir!("$OUT_DIR/static_common");

/// Serve embedded public static assets.
pub async fn serve_public(path: Option<Path<String>>) -> Response {
    serve_static(&STATIC_PUBLIC_ASSETS, path, "infra::assets::serve_public")
}

/// Serve embedded admin static assets.
pub async fn serve_admin(path: Option<Path<String>>) -> Response {
    serve_static(&STATIC_ADMIN_ASSETS, path, "infra::assets::serve_admin")
}

/// Serve embedded shared static assets.
pub async fn serve_common(path: Option<Path<String>>) -> Response {
    serve_static(&STATIC_SHARED_ASSETS, path, "infra::assets::serve_common")
}

fn serve_static(
    bundle: &'static Dir<'static>,
    path: Option<Path<String>>,
    source: &'static str,
) -> Response {
    let captured = path.map(|Path(value)| value);
    match resolve_asset(bundle, captured) {
        Ok(Some(asset)) => asset.into_response(),
        Ok(None) => not_found_response(source),
        Err(status) => rejected_response(source, status),
    }
}

fn not_found_response(source: &'static str) -> Response {
    let mut response = StatusCode::NOT_FOUND.into_response();
    ErrorReport::from_message(source, StatusCode::NOT_FOUND, "Static asset not found")
        .attach(&mut response);
    response
}

fn rejected_response(source: &'static str, status: StatusCode) -> Response {
    let mut response = status.into_response();
    ErrorReport::from_message(source, status, "Static asset request rejected")
        .attach(&mut response);
    response
}

struct Asset<'a> {
    contents: Cow<'a, [u8]>,
    mime: MimeGuess,
}

fn resolve_asset(
    bundle: &'static Dir<'static>,
    path: Option<String>,
) -> Result<Option<Asset<'static>>, StatusCode> {
    let mut candidate = path.unwrap_or_default();
    if candidate.starts_with('/') {
        candidate = candidate.trim_start_matches('/').to_string();
    }

    if candidate.is_empty() || candidate.ends_with('/') || candidate.contains("..") {
        // Avoid directory traversal and disallow directory listings.
        return Ok(None);
    }

    let Some(file) = bundle.get_file(&candidate) else {
        return Ok(None);
    };

    let mime = mime_guess::from_path(&candidate);
    let contents = Cow::Borrowed(file.contents());
    Ok(Some(Asset { contents, mime }))
}

impl IntoResponse for Asset<'static> {
    fn into_response(self) -> Response {
        let mime = self.mime.first_or_octet_stream();
        match self.contents {
            Cow::Borrowed(slice) => build_response(Bytes::from_static(slice), mime),
            Cow::Owned(bytes) => build_response(Bytes::from(bytes), mime),
        }
    }
}

fn build_response(bytes: Bytes, mime: Mime) -> Response {
    let len = bytes.len();
    let mut response = Response::new(Body::from(bytes));
    *response.status_mut() = StatusCode::OK;

    let headers = response.headers_mut();
    if let Ok(value) = HeaderValue::from_str(mime.as_ref()) {
        headers.insert(header::CONTENT_TYPE, value);
    }
    if let Ok(value) = HeaderValue::from_str(&len.to_string()) {
        headers.insert(header::CONTENT_LENGTH, value);
    }
    headers.insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("public, max-age=31536000, immutable"),
    );

    response
}
