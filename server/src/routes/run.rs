//! POST /api/run — Compile and execute Achronyme source code.
//!
//! Two modes:
//! - Without X-Ach-Session header: single-source mode (backward compatible)
//! - With X-Ach-Session header: workspace mode (reads from session workspace)

use std::time::Instant;

use axum::Json;
use serde::{Deserialize, Serialize};

use crate::error::ApiError;
use crate::pipeline;
use crate::sandbox::sandboxed;
use crate::session::SessionStore;

const RUN_TIMEOUT_SECS: u64 = 10;
const INSTRUCTION_BUDGET: u64 = 100_000_000;
const MAX_HEAP_BYTES: usize = 256 * 1024 * 1024;

#[derive(Deserialize)]
pub struct RunRequest {
    source: Option<String>,
}

#[derive(Serialize)]
pub struct RunResponse {
    success: bool,
    output: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
    time_ms: u64,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    proofs: Vec<crate::prove_handler::CapturedProof>,
}

pub async fn handler(
    axum::extract::State(store): axum::extract::State<SessionStore>,
    headers: axum::http::HeaderMap,
    Json(req): Json<RunRequest>,
) -> Result<Json<RunResponse>, ApiError> {
    let start = Instant::now();

    // Check for session header → workspace mode
    if let Some(session_val) = headers.get("X-Ach-Session") {
        let id: uuid::Uuid = session_val
            .to_str()
            .map_err(|_| ApiError::BadRequest("invalid session header".into()))?
            .parse()
            .map_err(|_| ApiError::BadRequest("invalid session id".into()))?;

        let workspace = store.get_workspace(id).map_err(ApiError::BadRequest)?;

        let result = sandboxed(
            move || crate::workspace::run_workspace(&workspace, INSTRUCTION_BUDGET, MAX_HEAP_BYTES),
            RUN_TIMEOUT_SECS,
        )
        .await?;

        return Ok(Json(RunResponse {
            success: result.success,
            output: crate::sanitize::scrub_paths(&result.output),
            error: crate::sanitize::scrub_option(result.error),
            time_ms: start.elapsed().as_millis() as u64,
            proofs: result.proofs,
        }));
    }

    // Single-source mode (backward compatible)
    let source = req
        .source
        .ok_or_else(|| ApiError::BadRequest("source is required".into()))?;
    if source.is_empty() {
        return Err(ApiError::BadRequest("source is empty".into()));
    }

    let result = sandboxed(
        move || pipeline::run_source(&source, INSTRUCTION_BUDGET, MAX_HEAP_BYTES),
        RUN_TIMEOUT_SECS,
    )
    .await?;

    Ok(Json(RunResponse {
        success: result.success,
        output: crate::sanitize::scrub_paths(&result.output),
        error: crate::sanitize::scrub_option(result.error),
        time_ms: start.elapsed().as_millis() as u64,
        proofs: result.proofs,
    }))
}
