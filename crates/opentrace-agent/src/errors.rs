// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AgntError {
    #[error("Not Found: {0}")]
    NotFound(String),

    #[error("Already Exists: {0}")]
    AlreadyExists(String),

    #[error("Bad Request: {0}")]
    BadRequest(String),

    #[error("Internal Error: {0}")]
    Internal(String),

    #[error("Other Error: {0}")]
    Other(String),
}

impl AgntError {
    /// 将任何 Display 类型包装为 AgntError::Other。
    pub(crate) fn other(msg: impl std::fmt::Display) -> Self {
        AgntError::Other(msg.to_string())
    }
}

impl IntoResponse for AgntError {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            AgntError::NotFound(msg) => (StatusCode::NOT_FOUND, msg.clone()),
            AgntError::AlreadyExists(msg) => (StatusCode::CONFLICT, msg.clone()),
            AgntError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg.clone()),
            AgntError::Internal(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg.clone()),
            AgntError::Other(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg.clone()),
        };

        (status, message).into_response()
    }
}
