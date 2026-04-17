//! File system endpoints for workspace management.

use axum::Json;
use serde::{Deserialize, Serialize};

use crate::error::ApiError;
use crate::session::{self, FileEntry, SessionStore};

use super::session::extract_session_id;

// ── Write ────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct WriteRequest {
    path: String,
    content: String,
}

#[derive(Serialize)]
pub struct OkResponse {
    ok: bool,
}

pub async fn write(
    axum::extract::State(store): axum::extract::State<SessionStore>,
    headers: axum::http::HeaderMap,
    Json(req): Json<WriteRequest>,
) -> Result<Json<OkResponse>, ApiError> {
    let id = extract_session_id(&headers)?;
    let workspace = store.get_workspace(id).map_err(ApiError::BadRequest)?;

    let file_path = session::validate_path(&workspace, &req.path).map_err(ApiError::BadRequest)?;

    store
        .check_write_limits(&workspace, &file_path, &req.content)
        .map_err(ApiError::BadRequest)?;

    std::fs::write(&file_path, &req.content)
        .map_err(|e| ApiError::Internal(format!("write: {e}")))?;

    Ok(Json(OkResponse { ok: true }))
}

// ── Read ─────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct ReadRequest {
    path: String,
}

#[derive(Serialize)]
pub struct ReadResponse {
    path: String,
    content: String,
}

pub async fn read(
    axum::extract::State(store): axum::extract::State<SessionStore>,
    headers: axum::http::HeaderMap,
    Json(req): Json<ReadRequest>,
) -> Result<Json<ReadResponse>, ApiError> {
    let id = extract_session_id(&headers)?;
    let workspace = store.get_workspace(id).map_err(ApiError::BadRequest)?;

    let file_path = session::validate_path(&workspace, &req.path).map_err(ApiError::BadRequest)?;

    if !file_path.exists() {
        return Err(ApiError::BadRequest(format!(
            "file not found: {}",
            req.path
        )));
    }

    let content = std::fs::read_to_string(&file_path)
        .map_err(|e| ApiError::Internal(format!("read: {e}")))?;

    Ok(Json(ReadResponse {
        path: req.path,
        content,
    }))
}

// ── Delete ───────────────────────────────────────────────

#[derive(Deserialize)]
pub struct DeleteRequest {
    path: String,
}

pub async fn delete(
    axum::extract::State(store): axum::extract::State<SessionStore>,
    headers: axum::http::HeaderMap,
    Json(req): Json<DeleteRequest>,
) -> Result<Json<OkResponse>, ApiError> {
    let id = extract_session_id(&headers)?;
    let workspace = store.get_workspace(id).map_err(ApiError::BadRequest)?;

    if req.path == "achronyme.toml" {
        return Err(ApiError::BadRequest("cannot delete achronyme.toml".into()));
    }

    // Try as file first, then as directory
    if let Ok(file_path) = session::validate_path(&workspace, &req.path) {
        if file_path.exists() {
            std::fs::remove_file(&file_path)
                .map_err(|e| ApiError::Internal(format!("delete: {e}")))?;
            return Ok(Json(OkResponse { ok: true }));
        }
    }

    // Try as directory
    if let Ok(dir_path) = session::validate_dir_path(&workspace, &req.path) {
        if dir_path.is_dir() {
            std::fs::remove_dir_all(&dir_path)
                .map_err(|e| ApiError::Internal(format!("delete dir: {e}")))?;
            return Ok(Json(OkResponse { ok: true }));
        }
    }

    Ok(Json(OkResponse { ok: true }))
}

// ── Mkdir ────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct MkdirRequest {
    path: String,
}

pub async fn mkdir(
    axum::extract::State(store): axum::extract::State<SessionStore>,
    headers: axum::http::HeaderMap,
    Json(req): Json<MkdirRequest>,
) -> Result<Json<OkResponse>, ApiError> {
    let id = extract_session_id(&headers)?;
    let workspace = store.get_workspace(id).map_err(ApiError::BadRequest)?;

    let dir_path =
        session::validate_dir_path(&workspace, &req.path).map_err(ApiError::BadRequest)?;

    std::fs::create_dir_all(&dir_path).map_err(|e| ApiError::Internal(format!("mkdir: {e}")))?;

    Ok(Json(OkResponse { ok: true }))
}

// ── List ─────────────────────────────────────────────────

#[derive(Serialize)]
pub struct ListResponse {
    files: Vec<FileEntry>,
}

pub async fn list(
    axum::extract::State(store): axum::extract::State<SessionStore>,
    headers: axum::http::HeaderMap,
) -> Result<Json<ListResponse>, ApiError> {
    let id = extract_session_id(&headers)?;
    let workspace = store.get_workspace(id).map_err(ApiError::BadRequest)?;

    let files = store.list_files(&workspace).map_err(ApiError::Internal)?;

    Ok(Json(ListResponse { files }))
}

// ── Rename ───────────────────────────────────────────────

#[derive(Deserialize)]
pub struct RenameRequest {
    from: String,
    to: String,
}

pub async fn rename(
    axum::extract::State(store): axum::extract::State<SessionStore>,
    headers: axum::http::HeaderMap,
    Json(req): Json<RenameRequest>,
) -> Result<Json<OkResponse>, ApiError> {
    let id = extract_session_id(&headers)?;
    let workspace = store.get_workspace(id).map_err(ApiError::BadRequest)?;

    if req.from == "achronyme.toml" {
        return Err(ApiError::BadRequest("cannot rename achronyme.toml".into()));
    }

    let from_path = session::validate_path(&workspace, &req.from).map_err(ApiError::BadRequest)?;
    let to_path = session::validate_path(&workspace, &req.to).map_err(ApiError::BadRequest)?;

    if !from_path.exists() {
        return Err(ApiError::BadRequest(format!(
            "file not found: {}",
            req.from
        )));
    }

    std::fs::rename(&from_path, &to_path)
        .map_err(|e| ApiError::Internal(format!("rename: {e}")))?;

    Ok(Json(OkResponse { ok: true }))
}
