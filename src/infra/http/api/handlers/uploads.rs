//! Uploads handlers

use axum::Json;
use axum::extract::{Extension, Multipart, Path, Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::application::api_keys::ApiPrincipal;
use crate::application::pagination::{PageRequest, UploadCursor};
use crate::application::repos::UploadQueryFilter;
use crate::domain::api_keys::ApiScope;
use crate::domain::entities::UploadRecord;

use super::{UploadListQuery, settings_to_api, upload_storage_to_api, upload_to_api};
use crate::infra::http::api::error::ApiError;
use crate::infra::http::api::models::UploadResponse;
use crate::infra::http::api::state::ApiState;

pub async fn list_uploads(
    State(state): State<ApiState>,
    Extension(principal): Extension<ApiPrincipal>,
    Query(query): Query<UploadListQuery>,
) -> Result<impl IntoResponse, ApiError> {
    principal
        .requires(ApiScope::UploadRead)
        .map_err(|_| ApiError::forbidden())?;
    let settings = state.settings.load().await.map_err(settings_to_api)?;
    let limit = query
        .limit
        .unwrap_or_else(|| settings.admin_page_size.max(1) as u32)
        .clamp(1, 100);

    let cursor = match query
        .cursor
        .as_deref()
        .map(UploadCursor::decode)
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

    let filter = UploadQueryFilter {
        content_type: query.content_type,
        month: query.month,
        search: query.search,
    };

    let page = state
        .uploads
        .list(&filter, PageRequest::new(limit, cursor))
        .await
        .map_err(upload_to_api)?;

    Ok(Json(page))
}

pub async fn get_upload(
    State(state): State<ApiState>,
    Extension(principal): Extension<ApiPrincipal>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    principal
        .requires(ApiScope::UploadRead)
        .map_err(|_| ApiError::forbidden())?;

    let upload = state.uploads.find_upload(id).await.map_err(upload_to_api)?;

    match upload {
        Some(record) => Ok(Json(record)),
        None => Err(ApiError::not_found("upload not found")),
    }
}

pub async fn upload_file(
    State(state): State<ApiState>,
    Extension(principal): Extension<ApiPrincipal>,
    mut multipart: Multipart,
) -> Result<impl IntoResponse, ApiError> {
    principal
        .requires(ApiScope::UploadWrite)
        .map_err(|_| ApiError::forbidden())?;
    let actor = ApiState::actor_label(&principal);

    let mut filename = None;
    let mut content_type = None;
    let mut data: Option<bytes::Bytes> = None;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|err| ApiError::bad_request("invalid multipart payload", Some(err.to_string())))?
    {
        if field.name() == Some("file") {
            filename = field.file_name().map(|s| s.to_string());
            content_type = field.content_type().map(|s| s.to_string());
            data = Some(field.bytes().await.map_err(|err| {
                ApiError::bad_request("failed to read upload", Some(err.to_string()))
            })?);
            break;
        }
    }

    let filename = filename.ok_or_else(|| ApiError::bad_request("missing file", None))?;
    let data = data.ok_or_else(|| ApiError::bad_request("missing file", None))?;
    let content_type = content_type.unwrap_or_else(|| "application/octet-stream".to_string());

    let stored = state
        .upload_storage
        .store(&filename, data)
        .await
        .map_err(upload_storage_to_api)?;

    let record = UploadRecord {
        id: Uuid::new_v4(),
        filename: filename.clone(),
        content_type: content_type.clone(),
        size_bytes: stored.size_bytes,
        checksum: stored.checksum.clone(),
        stored_path: stored.stored_path.clone(),
        metadata: crate::domain::uploads::UploadMetadata::default(),
        created_at: OffsetDateTime::now_utc(),
    };

    state
        .uploads
        .register_upload(&actor, record.clone())
        .await
        .map_err(upload_to_api)?;

    let response = UploadResponse {
        id: record.id,
        filename: record.filename,
        content_type: record.content_type,
        size_bytes: record.size_bytes,
        checksum: record.checksum,
        stored_path: record.stored_path,
        created_at: record.created_at,
    };

    Ok((StatusCode::CREATED, Json(response)))
}

pub async fn delete_upload(
    State(state): State<ApiState>,
    Extension(principal): Extension<ApiPrincipal>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    principal
        .requires(ApiScope::UploadWrite)
        .map_err(|_| ApiError::forbidden())?;
    let actor = ApiState::actor_label(&principal);

    let record = state
        .uploads
        .delete_upload(&actor, id)
        .await
        .map_err(upload_to_api)?;

    let _ = state.upload_storage.delete(&record.stored_path).await;

    Ok(StatusCode::NO_CONTENT)
}
