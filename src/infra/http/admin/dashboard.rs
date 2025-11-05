use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
};

use crate::presentation::{admin::views as admin_views, views::render_template_response};

use super::AdminState;

pub(super) async fn admin_dashboard(State(state): State<AdminState>) -> Response {
    let chrome = match state.chrome.load("/").await {
        Ok(chrome) => chrome,
        Err(err) => return err.into_response(),
    };

    let content = match state.dashboard.overview().await {
        Ok(view) => view,
        Err(err) => return err.into_response(),
    };

    let view = admin_views::AdminLayout::new(chrome, content);
    render_template_response(admin_views::AdminDashboardTemplate { view }, StatusCode::OK)
}
