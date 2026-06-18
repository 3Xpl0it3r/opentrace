// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.

use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde_json::json;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("Authentication error: {0}")]
    Auth(String),

    #[error("Forbidden: {0}")]
    Forbidden(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Bad request: {0}")]
    BadRequest(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            AppError::Database(rusqlite::Error::QueryReturnedNoRows) => {
                (StatusCode::NOT_FOUND, "Resource not found".to_string())
            }
            AppError::Database(rusqlite::Error::SqliteFailure(error, Some(message)))
                if error.code == rusqlite::ErrorCode::ConstraintViolation =>
            {
                (StatusCode::CONFLICT, message.clone())
            }
            AppError::Database(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
            AppError::Auth(e) => (StatusCode::UNAUTHORIZED, e.to_string()),
            AppError::Forbidden(e) => (StatusCode::FORBIDDEN, e.to_string()),
            AppError::NotFound(e) => (StatusCode::NOT_FOUND, e.to_string()),
            AppError::BadRequest(e) => (StatusCode::BAD_REQUEST, e.to_string()),
            AppError::Internal(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
        };

        (status, Json(json!({ "error": message }))).into_response()
    }
}
