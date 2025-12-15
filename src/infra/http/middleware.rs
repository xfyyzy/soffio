use std::time::Instant;

use axum::{body::Body, http::Request, middleware::Next, response::Response};
use tracing::{error, warn};
use uuid::Uuid;

use crate::{application::api_keys::ApiPrincipal, application::error::ErrorReport};

#[derive(Clone)]
pub struct RequestContext {
    pub request_id: String,
}

pub async fn set_request_context(mut request: Request<Body>, next: Next) -> Response {
    let request_id = Uuid::new_v4().to_string();
    let ctx = RequestContext {
        request_id: request_id.clone(),
    };
    request.extensions_mut().insert(ctx.clone());

    let mut response = next.run(request).await;
    response.extensions_mut().insert(ctx);
    response
}

pub async fn log_responses(request: Request<Body>, next: Next) -> Response {
    let method = request.method().clone();
    let uri = request.uri().clone();
    let start = Instant::now();

    let (api_key_id, api_scopes) = match request.extensions().get::<ApiPrincipal>() {
        Some(principal) => (
            Some(principal.key_id.to_string()),
            Some(
                principal
                    .scopes
                    .iter()
                    .map(|s| s.as_str())
                    .collect::<Vec<_>>()
                    .join(","),
            ),
        ),
        None => (None, None),
    };

    let request_id = request
        .extensions()
        .get::<RequestContext>()
        .map(|ctx| ctx.request_id.clone())
        .unwrap_or_default();

    let mut response = next.run(request).await;
    let status = response.status();

    if status.is_client_error() || status.is_server_error() {
        let elapsed_ms = start.elapsed().as_millis();
        let report = response.extensions_mut().remove::<ErrorReport>();
        let (source, messages) = match report {
            Some(report) => (report.source, report.messages),
            None => ("unknown", Vec::new()),
        };
        let detail = messages
            .first()
            .cloned()
            .unwrap_or_else(|| "no diagnostic available".to_string());

        if status.is_server_error() {
            error!(
                target = "soffio::http::response",
                status = status.as_u16(),
                method = %method,
                path = %uri.path(),
                query = uri.query().unwrap_or(""),
                elapsed_ms = elapsed_ms,
                source = source,
                detail = %detail,
                chain = ?messages,
                request_id = request_id,
                api_key_id = api_key_id.as_deref().unwrap_or(""),
                api_scopes = api_scopes.as_deref().unwrap_or(""),
                "request failed",
            );
        } else {
            warn!(
                target = "soffio::http::response",
                status = status.as_u16(),
                method = %method,
                path = %uri.path(),
                query = uri.query().unwrap_or(""),
                elapsed_ms = elapsed_ms,
                source = source,
                detail = %detail,
                chain = ?messages,
                request_id = request_id,
                api_key_id = api_key_id.as_deref().unwrap_or(""),
                api_scopes = api_scopes.as_deref().unwrap_or(""),
                "client request error",
            );
        }
    }

    response
}
