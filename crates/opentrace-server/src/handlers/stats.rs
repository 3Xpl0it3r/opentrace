use crate::errors::AppError;
use crate::state::AppState;
use axum::Json;
use axum::extract::State;
use serde::Serialize;

#[derive(Serialize)]
pub struct Stats {
    pub total_agents: i64,
    pub online_agents: i64,
    pub total_sinks: i64,
    pub healthy_sinks: i64,
}

pub async fn get_stats(State(state): State<AppState>) -> Result<Json<Stats>, AppError> {
    Ok(Json(Stats {
        total_agents: state.db.count_agents()?,
        online_agents: state.db.count_agents_by_status("online")?,
        total_sinks: state.db.count_sinks()?,
        healthy_sinks: state.db.count_sinks_by_status("healthy")?,
    }))
}
