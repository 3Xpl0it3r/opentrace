use crate::errors::AppError;
use crate::middleware::auth::{Claims, require_admin};
use crate::models::agent::SuccessResponse;
use crate::models::user::{CreateUser, User, UserRole};
use crate::state::AppState;
use axum::Json;
use axum::extract::{Path, State};

pub async fn list_users(
    State(state): State<AppState>,
    claims: Claims,
) -> Result<Json<Vec<User>>, AppError> {
    require_admin(&claims)?;
    let users = state.db.list_users()?;
    Ok(Json(users))
}

pub async fn create_user(
    State(state): State<AppState>,
    claims: Claims,
    Json(req): Json<CreateUser>,
) -> Result<Json<User>, AppError> {
    require_admin(&claims)?;
    let password_hash = bcrypt::hash(&req.password, bcrypt::DEFAULT_COST)
        .map_err(|e| AppError::Internal(e.to_string()))?;

    let role = req.role.unwrap_or(UserRole::Viewer);
    let user = state.db.create_user(&req.username, &password_hash, role)?;
    Ok(Json(user))
}

pub async fn delete_user(
    State(state): State<AppState>,
    claims: Claims,
    Path(id): Path<i64>,
) -> Result<Json<SuccessResponse>, AppError> {
    require_admin(&claims)?;
    let deleted = state.db.delete_user(id)?;
    if deleted {
        Ok(Json(SuccessResponse { success: true }))
    } else {
        Err(AppError::NotFound("User not found".to_string()))
    }
}
