use crate::errors::AppError;
use crate::middleware::auth::{Claims, require_editor};
use crate::models::agent::SuccessResponse;
use crate::models::group::{CreateGroup, Group};
use crate::state::AppState;
use axum::Json;
use axum::extract::{Path, State};

pub async fn list_groups(State(state): State<AppState>) -> Result<Json<Vec<Group>>, AppError> {
    let groups = state.db.list_groups()?;
    Ok(Json(groups))
}

pub async fn create_group(
    State(state): State<AppState>,
    claims: Claims,
    Json(req): Json<CreateGroup>,
) -> Result<Json<Group>, AppError> {
    require_editor(&claims)?;
    let group = state.db.create_group(&req)?;
    Ok(Json(group))
}

pub async fn delete_group(
    State(state): State<AppState>,
    claims: Claims,
    Path(id): Path<i64>,
) -> Result<Json<SuccessResponse>, AppError> {
    require_editor(&claims)?;
    let deleted = state.db.delete_group(id)?;
    if deleted {
        Ok(Json(SuccessResponse { success: true }))
    } else {
        Err(AppError::NotFound("Group not found".to_string()))
    }
}
