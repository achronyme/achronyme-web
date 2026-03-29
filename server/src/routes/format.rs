//! POST /api/format — Format source code (stub: returns source unchanged).

use axum::Json;
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct FormatRequest {
    source: String,
}

#[derive(Serialize)]
pub struct FormatResponse {
    formatted: String,
}

pub async fn handler(
    axum::extract::State(_store): axum::extract::State<crate::session::SessionStore>,
    Json(req): Json<FormatRequest>,
) -> Json<FormatResponse> {
    Json(FormatResponse {
        formatted: req.source,
    })
}
