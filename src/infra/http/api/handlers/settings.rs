//! Settings handlers

use axum::Json;
use axum::extract::{Extension, State};
use axum::response::IntoResponse;

use crate::application::admin::settings::UpdateSettingsCommand;
use crate::application::api_keys::ApiPrincipal;
use crate::domain::api_keys::ApiScope;

use super::settings_to_api;
use crate::infra::http::api::error::ApiError;
use crate::infra::http::api::models::SettingsPatchRequest;
use crate::infra::http::api::state::ApiState;

pub async fn get_settings(
    State(state): State<ApiState>,
    Extension(principal): Extension<ApiPrincipal>,
) -> Result<impl IntoResponse, ApiError> {
    principal
        .requires(ApiScope::SettingsRead)
        .map_err(|_| ApiError::forbidden())?;

    let settings = state.settings.load().await.map_err(settings_to_api)?;
    Ok(Json(settings))
}

pub async fn patch_settings(
    State(state): State<ApiState>,
    Extension(principal): Extension<ApiPrincipal>,
    Json(payload): Json<SettingsPatchRequest>,
) -> Result<impl IntoResponse, ApiError> {
    principal
        .requires(ApiScope::SettingsWrite)
        .map_err(|_| ApiError::forbidden())?;
    let actor = ApiState::actor_label(&principal);

    let mut current = state.settings.load().await.map_err(settings_to_api)?;

    if let Some(val) = payload.brand_title {
        current.brand_title = val;
    }
    if let Some(val) = payload.brand_href {
        current.brand_href = val;
    }
    if let Some(val) = payload.footer_copy {
        current.footer_copy = val;
    }
    if let Some(val) = payload.homepage_size {
        current.homepage_size = val;
    }
    if let Some(val) = payload.admin_page_size {
        current.admin_page_size = val;
    }
    if let Some(val) = payload.show_tag_aggregations {
        current.show_tag_aggregations = val;
    }
    if let Some(val) = payload.show_month_aggregations {
        current.show_month_aggregations = val;
    }
    if let Some(val) = payload.tag_filter_limit {
        current.tag_filter_limit = val;
    }
    if let Some(val) = payload.month_filter_limit {
        current.month_filter_limit = val;
    }
    if let Some(val) = payload.timezone {
        current.timezone = val
            .parse::<chrono_tz::Tz>()
            .map_err(|err| ApiError::bad_request("invalid timezone", Some(err.to_string())))?;
    }
    if let Some(val) = payload.meta_title {
        current.meta_title = val;
    }
    if let Some(val) = payload.meta_description {
        current.meta_description = val;
    }
    if let Some(val) = payload.og_title {
        current.og_title = val;
    }
    if let Some(val) = payload.og_description {
        current.og_description = val;
    }
    if let Some(val) = payload.public_site_url {
        current.public_site_url = val;
    }
    if let Some(val) = payload.global_toc_enabled {
        current.global_toc_enabled = val;
    }
    if let Some(val) = payload.favicon_svg {
        current.favicon_svg = val;
    }

    let command = UpdateSettingsCommand {
        homepage_size: current.homepage_size,
        admin_page_size: current.admin_page_size,
        show_tag_aggregations: current.show_tag_aggregations,
        show_month_aggregations: current.show_month_aggregations,
        tag_filter_limit: current.tag_filter_limit,
        month_filter_limit: current.month_filter_limit,
        global_toc_enabled: current.global_toc_enabled,
        brand_title: current.brand_title.clone(),
        brand_href: current.brand_href.clone(),
        footer_copy: current.footer_copy.clone(),
        public_site_url: current.public_site_url.clone(),
        favicon_svg: current.favicon_svg.clone(),
        timezone: current.timezone,
        meta_title: current.meta_title.clone(),
        meta_description: current.meta_description.clone(),
        og_title: current.og_title.clone(),
        og_description: current.og_description.clone(),
    };

    let updated = state
        .settings
        .update(&actor, command)
        .await
        .map_err(settings_to_api)?;

    Ok(Json(updated))
}
