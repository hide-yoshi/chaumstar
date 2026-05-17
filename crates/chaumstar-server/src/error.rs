//! HTTP error type for chaumstar-server handlers.

use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde_json::json;

#[derive(Debug)]
pub enum ApiError {
    NotFound,
    BadRequest(String),
    Conflict(String),
    Internal(String),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, code, message) = match self {
            ApiError::NotFound => (StatusCode::NOT_FOUND, "not_found", None),
            ApiError::BadRequest(m) => (StatusCode::BAD_REQUEST, "bad_request", Some(m)),
            ApiError::Conflict(m) => (StatusCode::CONFLICT, "conflict", Some(m)),
            ApiError::Internal(m) => (StatusCode::INTERNAL_SERVER_ERROR, "internal", Some(m)),
        };
        let body = match message {
            Some(m) => json!({ "error": code, "message": m }),
            None => json!({ "error": code }),
        };
        (status, Json(body)).into_response()
    }
}
