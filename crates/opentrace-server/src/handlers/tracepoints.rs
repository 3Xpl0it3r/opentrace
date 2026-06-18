use crate::db::tracepoints::{CreateTracepoint, Tracepoint, UpdateTracepoint};
use crate::errors::AppError;
use crate::handlers::agents::{start_agent_tracer_with_sink_name, stop_agent_tracer};
use crate::middleware::auth::{Claims, require_editor};
use crate::models::agent::SuccessResponse;
use crate::state::AppState;
use axum::{
    Json,
    extract::{Path, Query, State},
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Deserialize)]
pub struct TracepointsQuery {
    pub page: Option<i64>,
    pub page_size: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct PaginatedTracepoints {
    pub items: Vec<Tracepoint>,
    pub total: i64,
    pub page: i64,
    pub page_size: i64,
}

pub async fn list_tracepoints(
    State(state): State<AppState>,
    Path(agent_id): Path<i64>,
    Query(q): Query<TracepointsQuery>,
) -> Result<Json<PaginatedTracepoints>, AppError> {
    let page = q.page.unwrap_or(1).max(1);
    let page_size = q.page_size.unwrap_or(20).clamp(1, 100);
    let items = state.db.list_tracepoints(agent_id, page, page_size)?;
    let total = state.db.count_tracepoints_for_agent(agent_id)?;
    Ok(Json(PaginatedTracepoints {
        items,
        total,
        page,
        page_size,
    }))
}

pub async fn create_tracepoint(
    State(state): State<AppState>,
    claims: Claims,
    Path(agent_id): Path<i64>,
    Json(mut req): Json<CreateTracepoint>,
) -> Result<Json<Tracepoint>, AppError> {
    require_editor(&claims)?;
    let should_start = req.enabled.unwrap_or(false);
    req.enabled = Some(should_start);
    if should_start {
        let sink_name = sink_name_for_id(&state, req.sink_id)?;
        start_agent_tracer_with_sink_name(&state, agent_id, &req.name, sink_name).await?;
    }

    let tracepoint = match state.db.create_tracepoint(agent_id, &req) {
        Ok(tracepoint) => tracepoint,
        Err(err) => {
            if should_start {
                let _ = stop_agent_tracer(&state, agent_id, &req.name).await;
            }
            return Err(err.into());
        }
    };

    Ok(Json(tracepoint))
}

pub async fn update_tracepoint(
    State(state): State<AppState>,
    claims: Claims,
    Path((agent_id, tracepoint_id)): Path<(i64, i64)>,
    Json(req): Json<UpdateTracepoint>,
) -> Result<Json<Tracepoint>, AppError> {
    require_editor(&claims)?;
    let current = state.db.get_tracepoint_by_id(tracepoint_id)?;
    if current.agent_id != agent_id {
        return Err(AppError::NotFound("Tracepoint not found".to_string()));
    }

    let target_enabled = req.enabled.unwrap_or(current.enabled);
    let target_sink_id = req.sink_id.unwrap_or(current.sink_id);

    match (current.enabled, target_enabled) {
        (false, true) => {
            let sink_name = sink_name_for_id(&state, target_sink_id)?;
            start_agent_tracer_with_sink_name(&state, agent_id, &current.name, sink_name).await?;
            if let Err(err) = state.db.update_tracepoint(agent_id, tracepoint_id, &req) {
                let _ = stop_agent_tracer(&state, agent_id, &current.name).await;
                return Err(err.into());
            }
        }
        (true, false) => {
            stop_agent_tracer(&state, agent_id, &current.name).await?;
            if let Err(err) = state.db.update_tracepoint(agent_id, tracepoint_id, &req) {
                let _ = start_agent_tracer_with_sink_name(
                    &state,
                    agent_id,
                    &current.name,
                    sink_name_for_id(&state, current.sink_id)?,
                )
                .await;
                return Err(err.into());
            }
        }
        (true, true) if req.sink_id.is_some() && target_sink_id != current.sink_id => {
            let current_sink_name = sink_name_for_id(&state, current.sink_id)?;
            let target_sink_name = sink_name_for_id(&state, target_sink_id)?;

            stop_agent_tracer(&state, agent_id, &current.name).await?;
            if let Err(err) =
                start_agent_tracer_with_sink_name(&state, agent_id, &current.name, target_sink_name)
                    .await
            {
                let _ = start_agent_tracer_with_sink_name(
                    &state,
                    agent_id,
                    &current.name,
                    current_sink_name,
                )
                .await;
                return Err(err);
            }

            if let Err(err) = state.db.update_tracepoint(agent_id, tracepoint_id, &req) {
                let _ = stop_agent_tracer(&state, agent_id, &current.name).await;
                let _ = start_agent_tracer_with_sink_name(
                    &state,
                    agent_id,
                    &current.name,
                    sink_name_for_id(&state, current.sink_id)?,
                )
                .await;
                return Err(err.into());
            }
        }
        _ => {
            state.db.update_tracepoint(agent_id, tracepoint_id, &req)?;
        }
    }

    Ok(Json(state.db.get_tracepoint_by_id(tracepoint_id)?))
}

pub async fn delete_tracepoint(
    State(state): State<AppState>,
    claims: Claims,
    Path((agent_id, tracepoint_id)): Path<(i64, i64)>,
) -> Result<Json<SuccessResponse>, AppError> {
    require_editor(&claims)?;
    let tracepoint = state.db.get_tracepoint_by_id(tracepoint_id)?;
    if tracepoint.agent_id != agent_id {
        return Err(AppError::NotFound("Tracepoint not found".to_string()));
    }
    if tracepoint.enabled {
        stop_agent_tracer(&state, agent_id, &tracepoint.name).await?;
    }
    let deleted = match state.db.delete_tracepoint(agent_id, tracepoint_id) {
        Ok(deleted) => deleted,
        Err(err) => {
            if tracepoint.enabled {
                let _ = start_agent_tracer_with_sink_name(
                    &state,
                    agent_id,
                    &tracepoint.name,
                    sink_name_for_id(&state, tracepoint.sink_id)?,
                )
                .await;
            }
            return Err(err.into());
        }
    };
    if deleted {
        Ok(Json(SuccessResponse { success: true }))
    } else {
        Err(AppError::NotFound("Tracepoint not found".to_string()))
    }
}

fn sink_name_for_id(state: &AppState, sink_id: Option<i64>) -> Result<Option<String>, AppError> {
    let Some(sink_id) = sink_id else {
        return Ok(None);
    };

    state
        .db
        .get_sink_name_by_id(sink_id)?
        .map(Some)
        .ok_or_else(|| AppError::BadRequest(format!("sink '{sink_id}' not found")))
}
