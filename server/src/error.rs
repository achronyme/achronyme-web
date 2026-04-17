//! Unified API error type.

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde_json::json;

use crate::sanitize::scrub_paths;

pub enum ApiError {
    CompileError(String),
    Timeout,
    BadRequest(String),
    Internal(String),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        // User-visible error messages get scrubbed through `scrub_paths`
        // before leaving the server so internal paths like
        // `/tmp/ach-sessions/<uuid>/src/main.ach` don't leak to the client.
        // `Internal` already maps to a generic "internal server error"
        // response, so scrubbing there is defense-in-depth for the
        // server log only.
        let (status, code, message) = match self {
            ApiError::CompileError(msg) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "compile_error",
                scrub_paths(&msg),
            ),
            ApiError::Timeout => (
                StatusCode::REQUEST_TIMEOUT,
                "timeout",
                "execution timed out".to_string(),
            ),
            ApiError::BadRequest(msg) => {
                (StatusCode::BAD_REQUEST, "bad_request", scrub_paths(&msg))
            }
            ApiError::Internal(msg) => {
                tracing::error!("internal error: {}", scrub_paths(&msg));
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "internal_error",
                    "internal server error".to_string(),
                )
            }
        };

        let body = json!({
            "error": message,
            "code": code,
        });

        (status, axum::Json(body)).into_response()
    }
}
