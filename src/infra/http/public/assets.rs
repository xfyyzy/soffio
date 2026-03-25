use std::io::ErrorKind;

use axum::{
    body::Body,
    extract::{Path, State},
    http::{
        HeaderValue, StatusCode,
        header::{CACHE_CONTROL, CONTENT_LENGTH, CONTENT_TYPE},
    },
    response::{IntoResponse, Response},
};
use bytes::Bytes;
use tracing::error;

use crate::{
    application::{error::HttpError, repos::SettingsRepo},
    infra::uploads::UploadStorageError,
};

use super::HttpState;

pub(super) async fn serve_upload(
    State(state): State<HttpState>,
    Path(path): Path<String>,
) -> Response {
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

pub(super) async fn public_health(State(state): State<HttpState>) -> Response {
    super::super::db_health_response(state.db.health_check().await)
}

pub(super) async fn favicon(State(state): State<HttpState>) -> Response {
    crate::cache::deps::record(crate::cache::EntityKey::SiteSettings);

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
