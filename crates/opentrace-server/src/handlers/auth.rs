use crate::errors::AppError;
use crate::middleware::auth::{Claims, create_token};
use crate::models::user::{LoginRequest, LoginResponse};
use crate::state::AppState;
use axum::{Json, extract::State};

pub async fn login(
    State(state): State<AppState>,
    Json(req): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, AppError> {
    let user = state
        .db
        .get_user_by_username(&req.username)
        .map_err(|_e| AppError::Auth("Invalid credentials".to_string()))?;

    let valid = bcrypt::verify(&req.password, &user.password_hash)
        .map_err(|e| AppError::Internal(e.to_string()))?;

    if !valid {
        return Err(AppError::Auth("Invalid credentials".to_string()));
    }

    let token = create_token(
        user.id,
        &user.username,
        user.role.as_str(),
        &state.jwt_secret,
    )
    .map_err(|e| AppError::Internal(e.to_string()))?;

    Ok(Json(LoginResponse { token, user }))
}

pub async fn me(
    State(state): State<AppState>,
    claims: Claims,
) -> Result<Json<crate::models::user::User>, AppError> {
    let user = state.db.get_user_by_id(claims.sub)?;
    Ok(Json(user))
}
