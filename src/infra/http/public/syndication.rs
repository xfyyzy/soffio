use axum::{
    body::Body,
    extract::State,
    http::{StatusCode, header::CONTENT_TYPE},
    response::{IntoResponse, Response},
};

use crate::application::error::HttpError;

use super::HttpState;

pub(super) async fn sitemap(State(state): State<HttpState>) -> Response {
    match state.sitemap.sitemap_xml().await {
        Ok(body) => xml_response(body, "application/xml"),
        Err(err) => HttpError::new(
            "infra::http::public::sitemap",
            StatusCode::INTERNAL_SERVER_ERROR,
            "Failed to generate sitemap",
            err.to_string(),
        )
        .into_response(),
    }
}

pub(super) async fn rss_feed(State(state): State<HttpState>) -> Response {
    match state.syndication.rss_feed().await {
        Ok(body) => xml_response(body, "application/rss+xml"),
        Err(err) => HttpError::new(
            "infra::http::public::rss",
            StatusCode::INTERNAL_SERVER_ERROR,
            "Failed to generate RSS feed",
            err.to_string(),
        )
        .into_response(),
    }
}

pub(super) async fn atom_feed(State(state): State<HttpState>) -> Response {
    match state.syndication.atom_feed().await {
        Ok(body) => xml_response(body, "application/atom+xml"),
        Err(err) => HttpError::new(
            "infra::http::public::atom",
            StatusCode::INTERNAL_SERVER_ERROR,
            "Failed to generate Atom feed",
            err.to_string(),
        )
        .into_response(),
    }
}

pub(super) async fn robots_txt(State(state): State<HttpState>) -> Response {
    match state.sitemap.robots_txt().await {
        Ok(body) => plain_response(body),
        Err(err) => HttpError::new(
            "infra::http::public::robots",
            StatusCode::INTERNAL_SERVER_ERROR,
            "Failed to generate robots.txt",
            err.to_string(),
        )
        .into_response(),
    }
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
