//! Unified API error type.

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde_json::json;

pub enum ApiError {
    CompileError(String),
    RuntimeError(String),
    Timeout,
    BadRequest(String),
    Internal(String),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, code, message) = match self {
            ApiError::CompileError(msg) => (StatusCode::UNPROCESSABLE_ENTITY, "compile_error", msg),
            ApiError::RuntimeError(msg) => (StatusCode::UNPROCESSABLE_ENTITY, "runtime_error", msg),
            ApiError::Timeout => (
                StatusCode::REQUEST_TIMEOUT,
                "timeout",
                "execution timed out".to_string(),
            ),
            ApiError::BadRequest(msg) => (StatusCode::BAD_REQUEST, "bad_request", msg),
            ApiError::Internal(msg) => {
                tracing::error!("internal error: {msg}");
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
