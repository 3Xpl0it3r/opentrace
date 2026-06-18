// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.

use axum::{
    extract::{FromRef, FromRequestParts, Request, State},
    http::header,
    http::request::Parts,
    middleware::Next,
    response::Response,
};
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};

use crate::errors::AppError;
use crate::state::AppState;

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: i64,
    pub username: String,
    pub role: String,
    pub exp: usize,
}

impl<S> FromRequestParts<S> for Claims
where
    S: Send + Sync,
    AppState: FromRef<S>,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let auth_header = parts
            .headers
            .get(header::AUTHORIZATION)
            .and_then(|value| value.to_str().ok());

        let token = auth_header
            .and_then(|value| value.strip_prefix("Bearer "))
            .ok_or_else(|| AppError::Auth("Missing authorization header".to_string()))?;

        let state = AppState::from_ref(state);
        let claims = verify_token(token, &state.jwt_secret)
            .map_err(|e| AppError::Auth(format!("Invalid token: {}", e)))?;

        Ok(claims)
    }
}

pub fn create_token(
    user_id: i64,
    username: &str,
    role: &str,
    secret: &str,
) -> Result<String, jsonwebtoken::errors::Error> {
    let expiration = chrono::Utc::now()
        .checked_add_signed(chrono::Duration::hours(24))
        .expect("valid timestamp")
        .timestamp() as usize;

    let claims = Claims {
        sub: user_id,
        username: username.to_string(),
        role: role.to_string(),
        exp: expiration,
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
}

pub fn verify_token(token: &str, secret: &str) -> Result<Claims, jsonwebtoken::errors::Error> {
    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::default(),
    )?;
    Ok(token_data.claims)
}

pub fn require_admin(claims: &Claims) -> Result<(), AppError> {
    if claims.role == "admin" {
        Ok(())
    } else {
        Err(AppError::Forbidden("Admin permission required".to_string()))
    }
}

pub fn require_editor(claims: &Claims) -> Result<(), AppError> {
    match claims.role.as_str() {
        "admin" | "editor" => Ok(()),
        _ => Err(AppError::Forbidden(
            "Editor permission required".to_string(),
        )),
    }
}

pub async fn auth_middleware(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Result<Response, AppError> {
    let auth_header = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok());

    let token = auth_header
        .and_then(|value| value.strip_prefix("Bearer "))
        .ok_or_else(|| AppError::Auth("Missing authorization header".to_string()))?;

    let _claims = verify_token(token, &state.jwt_secret)
        .map_err(|e| AppError::Auth(format!("Invalid token: {}", e)))?;

    Ok(next.run(request).await)
}
