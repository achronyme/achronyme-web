//! Session management endpoints.

use axum::Json;
use serde::{Deserialize, Serialize};

use crate::error::ApiError;
use crate::session::{FileEntry, SessionStore};

#[derive(Deserialize)]
pub struct CreateRequest {
    template: Option<String>,
}

#[derive(Serialize)]
pub struct CreateResponse {
    session_id: String,
    files: Vec<FileEntry>,
}

pub async fn create(
    axum::extract::State(store): axum::extract::State<SessionStore>,
    Json(req): Json<CreateRequest>,
) -> Result<Json<CreateResponse>, ApiError> {
    let (id, workspace) = store
        .create(req.template.as_deref())
        .map_err(|e| ApiError::BadRequest(e))?;

    let files = store
        .list_files(&workspace)
        .map_err(|e| ApiError::Internal(e))?;

    Ok(Json(CreateResponse {
        session_id: id.to_string(),
        files,
    }))
}

#[derive(Serialize)]
pub struct DeleteResponse {
    ok: bool,
}

pub async fn delete(
    axum::extract::State(store): axum::extract::State<SessionStore>,
    headers: axum::http::HeaderMap,
) -> Result<Json<DeleteResponse>, ApiError> {
    let id = extract_session_id(&headers)?;
    store.delete(id).map_err(|e| ApiError::Internal(e))?;
    Ok(Json(DeleteResponse { ok: true }))
}

/// Extract session UUID from X-Ach-Session header.
pub fn extract_session_id(
    headers: &axum::http::HeaderMap,
) -> Result<uuid::Uuid, ApiError> {
    let val = headers
        .get("X-Ach-Session")
        .ok_or_else(|| ApiError::BadRequest("missing X-Ach-Session header".into()))?
        .to_str()
        .map_err(|_| ApiError::BadRequest("invalid session header".into()))?;
    val.parse::<uuid::Uuid>()
        .map_err(|_| ApiError::BadRequest("invalid session id".into()))
}
