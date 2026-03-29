//! POST /api/compile — Check source code for errors without executing.

use axum::Json;
use serde::{Deserialize, Serialize};

use crate::error::ApiError;
use crate::pipeline;
use crate::sandbox::sandboxed;

const COMPILE_TIMEOUT_SECS: u64 = 5;

#[derive(Deserialize)]
pub struct CompileRequest {
    source: String,
}

#[derive(Serialize)]
pub struct CompileResponse {
    success: bool,
    diagnostics: Vec<pipeline::DiagnosticInfo>,
}

pub async fn handler(
    axum::extract::State(_store): axum::extract::State<crate::session::SessionStore>,
    Json(req): Json<CompileRequest>,
) -> Result<Json<CompileResponse>, ApiError> {
    let source = req.source;

    let result = sandboxed(move || pipeline::check_source(&source), COMPILE_TIMEOUT_SECS).await?;

    Ok(Json(CompileResponse {
        success: result.success,
        diagnostics: result.diagnostics,
    }))
}
