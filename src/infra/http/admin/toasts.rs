use axum::{
    extract::Form,
    response::{IntoResponse, Response},
};
use serde::Deserialize;

use crate::application::error::HttpError;

use super::shared::{Toast, push_toasts};

#[derive(Debug, Deserialize)]
pub(super) struct AdminToastForm {
    kind: String,
    message: String,
}

pub(super) async fn admin_toast(Form(form): Form<AdminToastForm>) -> Response {
    let toast = match form.kind.as_str() {
        "success" => Toast::success(form.message),
        "error" => Toast::error(form.message),
        other => {
            return HttpError::new(
                "infra::http::admin_toasts",
                axum::http::StatusCode::BAD_REQUEST,
                "Invalid toast kind",
                format!("Unsupported toast kind `{other}`"),
            )
            .into_response();
        }
    };

    let mut stream = crate::application::stream::StreamBuilder::new();
    if let Err(err) = push_toasts(&mut stream, &[toast]) {
        return err.into_response();
    }
    stream.into_response()
}
