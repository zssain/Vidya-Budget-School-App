//! Error handling. Unexpected failures are logged server-side and answered with a
//! generic 500 (Step 4: error messages never reveal whether other schools exist).

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;

/// An unexpected internal failure. The detail is logged, never sent to the client.
pub struct AppError(pub String);

impl<E: std::fmt::Display> From<E> for AppError {
    fn from(e: E) -> Self {
        AppError(e.to_string())
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        tracing::error!(error = %self.0, "internal error");
        (StatusCode::INTERNAL_SERVER_ERROR, "Something went wrong. Please try again.").into_response()
    }
}

/// A generic JSON error body `{ "error": CODE }` for the licence API.
pub fn json_err(status: StatusCode, code: &str) -> Response {
    (status, Json(serde_json::json!({ "error": code }))).into_response()
}
