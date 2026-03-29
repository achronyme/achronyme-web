//! POST /api/run — Compile and execute Achronyme source code.

use std::time::Instant;

use axum::Json;
use serde::{Deserialize, Serialize};

use crate::error::ApiError;
use crate::pipeline;
use crate::sandbox::sandboxed;

const RUN_TIMEOUT_SECS: u64 = 10;
const INSTRUCTION_BUDGET: u64 = 100_000_000;
const MAX_HEAP_BYTES: usize = 256 * 1024 * 1024;

#[derive(Deserialize)]
pub struct RunRequest {
    source: String,
}

#[derive(Serialize)]
pub struct RunResponse {
    success: bool,
    output: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
    time_ms: u64,
}

pub async fn handler(Json(req): Json<RunRequest>) -> Result<Json<RunResponse>, ApiError> {
    if req.source.is_empty() {
        return Err(ApiError::BadRequest("source is empty".into()));
    }

    let source = req.source;
    let start = Instant::now();

    let result = sandboxed(
        move || pipeline::run_source(&source, INSTRUCTION_BUDGET, MAX_HEAP_BYTES),
        RUN_TIMEOUT_SECS,
    )
    .await?;

    Ok(Json(RunResponse {
        success: result.success,
        output: result.output,
        error: result.error,
        time_ms: start.elapsed().as_millis() as u64,
    }))
}
