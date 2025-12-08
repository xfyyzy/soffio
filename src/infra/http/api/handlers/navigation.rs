//! Navigation handlers

use axum::Json;
use axum::extract::{Extension, Path, Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use uuid::Uuid;

use crate::application::admin::navigation::{
    CreateNavigationItemCommand, UpdateNavigationItemCommand,
};
use crate::application::api_keys::ApiPrincipal;
use crate::application::pagination::{NavigationCursor, PageRequest};
use crate::application::repos::NavigationQueryFilter;
use crate::domain::api_keys::ApiScope;

use super::{NavigationListQuery, nav_to_api, settings_to_api};
use crate::infra::http::api::error::ApiError;
use crate::infra::http::api::models::*;
use crate::infra::http::api::state::ApiState;

pub async fn list_navigation(
    State(state): State<ApiState>,
    Extension(principal): Extension<ApiPrincipal>,
    Query(query): Query<NavigationListQuery>,
) -> Result<impl IntoResponse, ApiError> {
    principal
        .requires(ApiScope::NavigationRead)
        .map_err(|_| ApiError::forbidden())?;
    let settings = state.settings.load().await.map_err(settings_to_api)?;
    let limit = query
        .limit
        .unwrap_or_else(|| settings.admin_page_size.max(1) as u32)
        .clamp(1, 100);

    let cursor = match query
        .cursor
        .as_deref()
        .map(NavigationCursor::decode)
        .transpose()
    {
        Ok(cursor) => cursor,
        Err(err) => {
            return Err(ApiError::bad_request(
                "invalid cursor",
                Some(err.to_string()),
            ));
        }
    };

    let filter = NavigationQueryFilter {
        search: query.search,
    };

    let page = state
        .navigation
        .list(query.visible, &filter, PageRequest::new(limit, cursor))
        .await
        .map_err(nav_to_api)?;

    Ok(Json(page))
}

pub async fn get_navigation_item(
    State(state): State<ApiState>,
    Extension(principal): Extension<ApiPrincipal>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    principal
        .requires(ApiScope::NavigationRead)
        .map_err(|_| ApiError::forbidden())?;

    let item = state.navigation.find_by_id(id).await.map_err(nav_to_api)?;

    match item {
        Some(record) => Ok(Json(record)),
        None => Err(ApiError::not_found("navigation item not found")),
    }
}

pub async fn create_navigation(
    State(state): State<ApiState>,
    Extension(principal): Extension<ApiPrincipal>,
    Json(payload): Json<NavigationCreateRequest>,
) -> Result<impl IntoResponse, ApiError> {
    principal
        .requires(ApiScope::NavigationWrite)
        .map_err(|_| ApiError::forbidden())?;
    let actor = ApiState::actor_label(&principal);

    let command = CreateNavigationItemCommand {
        label: payload.label,
        destination_type: payload.destination_type,
        destination_page_id: payload.destination_page_id,
        destination_url: payload.destination_url,
        sort_order: payload.sort_order,
        visible: payload.visible,
        open_in_new_tab: payload.open_in_new_tab,
    };

    let record = state
        .navigation
        .create_item(&actor, command)
        .await
        .map_err(nav_to_api)?;

    Ok((StatusCode::CREATED, Json(record)))
}

pub async fn update_navigation(
    State(state): State<ApiState>,
    Extension(principal): Extension<ApiPrincipal>,
    Path(id): Path<Uuid>,
    Json(payload): Json<NavigationUpdateRequest>,
) -> Result<impl IntoResponse, ApiError> {
    principal
        .requires(ApiScope::NavigationWrite)
        .map_err(|_| ApiError::forbidden())?;
    let actor = ApiState::actor_label(&principal);

    let command = UpdateNavigationItemCommand {
        id,
        label: payload.label,
        destination_type: payload.destination_type,
        destination_page_id: payload.destination_page_id,
        destination_url: payload.destination_url,
        sort_order: payload.sort_order,
        visible: payload.visible,
        open_in_new_tab: payload.open_in_new_tab,
    };

    let record = state
        .navigation
        .update_item(&actor, command)
        .await
        .map_err(nav_to_api)?;

    Ok(Json(record))
}

pub async fn update_navigation_label(
    State(state): State<ApiState>,
    Extension(principal): Extension<ApiPrincipal>,
    Path(id): Path<Uuid>,
    Json(payload): Json<NavigationLabelRequest>,
) -> Result<impl IntoResponse, ApiError> {
    principal
        .requires(ApiScope::NavigationWrite)
        .map_err(|_| ApiError::forbidden())?;
    let actor = ApiState::actor_label(&principal);

    let existing = state
        .navigation
        .find_by_id(id)
        .await
        .map_err(nav_to_api)?
        .ok_or_else(|| ApiError::not_found("navigation not found"))?;

    let command = UpdateNavigationItemCommand {
        id,
        label: payload.label,
        destination_type: existing.destination_type,
        destination_page_id: existing.destination_page_id,
        destination_url: existing.destination_url.clone(),
        sort_order: existing.sort_order,
        visible: existing.visible,
        open_in_new_tab: existing.open_in_new_tab,
    };

    let record = state
        .navigation
        .update_item(&actor, command)
        .await
        .map_err(nav_to_api)?;

    Ok(Json(record))
}

pub async fn update_navigation_destination(
    State(state): State<ApiState>,
    Extension(principal): Extension<ApiPrincipal>,
    Path(id): Path<Uuid>,
    Json(payload): Json<NavigationDestinationRequest>,
) -> Result<impl IntoResponse, ApiError> {
    principal
        .requires(ApiScope::NavigationWrite)
        .map_err(|_| ApiError::forbidden())?;
    let actor = ApiState::actor_label(&principal);

    let existing = state
        .navigation
        .find_by_id(id)
        .await
        .map_err(nav_to_api)?
        .ok_or_else(|| ApiError::not_found("navigation not found"))?;

    let command = UpdateNavigationItemCommand {
        id,
        label: existing.label.clone(),
        destination_type: payload.destination_type,
        destination_page_id: payload.destination_page_id,
        destination_url: payload.destination_url,
        sort_order: existing.sort_order,
        visible: existing.visible,
        open_in_new_tab: existing.open_in_new_tab,
    };

    let record = state
        .navigation
        .update_item(&actor, command)
        .await
        .map_err(nav_to_api)?;

    Ok(Json(record))
}

pub async fn update_navigation_sort_order(
    State(state): State<ApiState>,
    Extension(principal): Extension<ApiPrincipal>,
    Path(id): Path<Uuid>,
    Json(payload): Json<NavigationSortOrderRequest>,
) -> Result<impl IntoResponse, ApiError> {
    principal
        .requires(ApiScope::NavigationWrite)
        .map_err(|_| ApiError::forbidden())?;
    let actor = ApiState::actor_label(&principal);

    let existing = state
        .navigation
        .find_by_id(id)
        .await
        .map_err(nav_to_api)?
        .ok_or_else(|| ApiError::not_found("navigation not found"))?;

    let command = UpdateNavigationItemCommand {
        id,
        label: existing.label.clone(),
        destination_type: existing.destination_type,
        destination_page_id: existing.destination_page_id,
        destination_url: existing.destination_url.clone(),
        sort_order: payload.sort_order,
        visible: existing.visible,
        open_in_new_tab: existing.open_in_new_tab,
    };

    let record = state
        .navigation
        .update_item(&actor, command)
        .await
        .map_err(nav_to_api)?;

    Ok(Json(record))
}

pub async fn update_navigation_visibility(
    State(state): State<ApiState>,
    Extension(principal): Extension<ApiPrincipal>,
    Path(id): Path<Uuid>,
    Json(payload): Json<NavigationVisibilityRequest>,
) -> Result<impl IntoResponse, ApiError> {
    principal
        .requires(ApiScope::NavigationWrite)
        .map_err(|_| ApiError::forbidden())?;
    let actor = ApiState::actor_label(&principal);

    let existing = state
        .navigation
        .find_by_id(id)
        .await
        .map_err(nav_to_api)?
        .ok_or_else(|| ApiError::not_found("navigation not found"))?;

    let command = UpdateNavigationItemCommand {
        id,
        label: existing.label.clone(),
        destination_type: existing.destination_type,
        destination_page_id: existing.destination_page_id,
        destination_url: existing.destination_url.clone(),
        sort_order: existing.sort_order,
        visible: payload.visible,
        open_in_new_tab: existing.open_in_new_tab,
    };

    let record = state
        .navigation
        .update_item(&actor, command)
        .await
        .map_err(nav_to_api)?;

    Ok(Json(record))
}

pub async fn update_navigation_open_in_new_tab(
    State(state): State<ApiState>,
    Extension(principal): Extension<ApiPrincipal>,
    Path(id): Path<Uuid>,
    Json(payload): Json<NavigationOpenInNewTabRequest>,
) -> Result<impl IntoResponse, ApiError> {
    principal
        .requires(ApiScope::NavigationWrite)
        .map_err(|_| ApiError::forbidden())?;
    let actor = ApiState::actor_label(&principal);

    let existing = state
        .navigation
        .find_by_id(id)
        .await
        .map_err(nav_to_api)?
        .ok_or_else(|| ApiError::not_found("navigation not found"))?;

    let command = UpdateNavigationItemCommand {
        id,
        label: existing.label.clone(),
        destination_type: existing.destination_type,
        destination_page_id: existing.destination_page_id,
        destination_url: existing.destination_url.clone(),
        sort_order: existing.sort_order,
        visible: existing.visible,
        open_in_new_tab: payload.open_in_new_tab,
    };

    let record = state
        .navigation
        .update_item(&actor, command)
        .await
        .map_err(nav_to_api)?;

    Ok(Json(record))
}

pub async fn delete_navigation(
    State(state): State<ApiState>,
    Extension(principal): Extension<ApiPrincipal>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    principal
        .requires(ApiScope::NavigationWrite)
        .map_err(|_| ApiError::forbidden())?;
    let actor = ApiState::actor_label(&principal);

    state
        .navigation
        .delete_item(&actor, id)
        .await
        .map_err(nav_to_api)?;

    Ok(StatusCode::NO_CONTENT)
}
