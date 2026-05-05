//! POST /api/compile — Check source code for errors without executing.
//!
//! Two modes:
//! - Without X-Ach-Session header: single-source `.ach` mode (back-compat).
//! - With X-Ach-Session header: workspace mode. Reads `[project] entry`
//!   from `achronyme.toml`. Dispatches to the circom front-end if the
//!   entry ends in `.circom`, otherwise to the `.ach` checker.

use std::path::PathBuf;

use axum::Json;
use serde::{Deserialize, Serialize};

use crate::error::ApiError;
use crate::pipeline;
use crate::sandbox::sandboxed;

const COMPILE_TIMEOUT_SECS: u64 = 5;
/// `.circom` lowering can be substantially slower than `.ach` parsing
/// (heavy templates like SHA-256(64) run in tens of seconds), so this
/// path takes the same ceiling as `/api/circuit` rather than the tight
/// 5 s `.ach` budget. Circuits that exceed this need `/api/circuit`.
const COMPILE_CIRCOM_TIMEOUT_SECS: u64 = 30;

#[derive(Deserialize)]
pub struct CompileRequest {
    #[serde(default)]
    source: Option<String>,
}

#[derive(Serialize)]
pub struct CompileResponse {
    success: bool,
    diagnostics: Vec<pipeline::DiagnosticInfo>,
}

pub async fn handler(
    axum::extract::State(store): axum::extract::State<crate::session::SessionStore>,
    headers: axum::http::HeaderMap,
    Json(req): Json<CompileRequest>,
) -> Result<Json<CompileResponse>, ApiError> {
    if let Some(session_val) = headers.get("X-Ach-Session") {
        let id: uuid::Uuid = session_val
            .to_str()
            .map_err(|_| ApiError::BadRequest("invalid session header".into()))?
            .parse()
            .map_err(|_| ApiError::BadRequest("invalid session id".into()))?;

        let workspace = store.get_workspace(id).map_err(ApiError::BadRequest)?;
        let config =
            crate::workspace::load_workspace_config(&workspace).map_err(ApiError::BadRequest)?;

        let is_circom = config
            .entry
            .extension()
            .and_then(|e| e.to_str())
            .map(|s| s.eq_ignore_ascii_case("circom"))
            .unwrap_or(false);

        return if is_circom {
            let entry = config.entry.clone();
            let libs = config.circom_libs.clone();
            let result = sandboxed(
                move || compile_circom_workspace(&entry, &libs),
                COMPILE_CIRCOM_TIMEOUT_SECS,
            )
            .await?;
            Ok(Json(result))
        } else {
            let source = std::fs::read_to_string(&config.entry).map_err(|e| {
                ApiError::BadRequest(format!("cannot read entry {}: {e}", config.entry.display()))
            })?;
            let result = sandboxed(
                move || pipeline::check_source(&source),
                COMPILE_TIMEOUT_SECS,
            )
            .await?;
            Ok(Json(CompileResponse {
                success: result.success,
                diagnostics: result.diagnostics,
            }))
        };
    }

    // Single-source mode: .ach only. circom needs filesystem `include`
    // resolution, which only workspace mode provides.
    let source = req
        .source
        .ok_or_else(|| ApiError::BadRequest("source is required".into()))?;
    if source.is_empty() {
        return Err(ApiError::BadRequest("source is empty".into()));
    }

    let result = sandboxed(
        move || pipeline::check_source(&source),
        COMPILE_TIMEOUT_SECS,
    )
    .await?;

    Ok(Json(CompileResponse {
        success: result.success,
        diagnostics: result.diagnostics,
    }))
}

fn compile_circom_workspace(entry: &std::path::Path, libs: &[PathBuf]) -> CompileResponse {
    match crate::circom_pipeline::compile_circom(entry, libs) {
        Ok(compiled) => CompileResponse {
            success: true,
            diagnostics: crate::circom_pipeline::diagnostics_to_pipeline_format(&compiled.warnings),
        },
        Err(diags) => CompileResponse {
            success: false,
            diagnostics: crate::circom_pipeline::diagnostics_to_pipeline_format(&diags),
        },
    }
}
