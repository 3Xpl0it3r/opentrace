// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.

use std::sync::Arc;

use axum::body::Body;
use axum::extract::State;
use axum::http::{HeaderMap, Request, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};

const AUTHORIZATION_KEYWORKD: &str = "authorization";

#[derive(Clone)]
pub struct AuthState {
    pub bearer_token: Arc<str>,
}

fn write_401(error_type: &str, message: &str) -> Response {
    (
        StatusCode::UNAUTHORIZED,
        [("Authenticate", format!("Bearer error={}", error_type))],
        message.to_string(),
    )
        .into_response()
}

pub(super) async fn bearer_auth_middleware(
    State(auth_state): State<AuthState>,
    headers: HeaderMap,
    request: Request<Body>,
    next: Next,
) -> Response {
    let auth_header = headers
        .get(AUTHORIZATION_KEYWORKD)
        .and_then(|v| v.to_str().ok());

    match auth_header {
        Some(auth) if auth.starts_with("Bearer ") => {
            let token = auth.trim_start_matches("Bearer ");
            if token == auth_state.bearer_token.as_ref() {
                next.run(request).await
            } else {
                write_401("invalid_token", "Unauthorized: Bearer token is invalid")
            }
        }
        _ => write_401("missing_token", "Unauthorized: Bearer token required"),
    }
}
