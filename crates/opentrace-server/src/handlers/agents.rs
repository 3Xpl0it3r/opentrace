use crate::agent_url::build_agent_url;
use crate::db::Database;
use crate::db::tracepoints::CreateTracepoint;
use crate::errors::AppError;
use crate::handlers::sinks::{create_agent_sink, rollback_agent_sinks};
use crate::middleware::auth::{Claims, require_editor};
use crate::models::agent::{
    Agent, AgentSystemInfo, CreateAgent, SuccessResponse, TracerStartRequest, UpdateAgent,
};
use crate::state::AppState;
use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct AgentQuery {
    pub group: Option<i64>,
    pub tag: Option<String>,
}

pub async fn list_agents(
    State(state): State<AppState>,
    Query(query): Query<AgentQuery>,
) -> Result<Json<Vec<Agent>>, AppError> {
    let agents = state.db.list_agents(query.group, query.tag.as_deref())?;
    Ok(Json(agents))
}

pub async fn get_agent(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<Agent>, AppError> {
    let agent = state.db.get_agent_by_id(id)?;
    Ok(Json(agent))
}

pub async fn create_agent(
    State(state): State<AppState>,
    claims: Claims,
    Json(req): Json<CreateAgent>,
) -> Result<Json<Agent>, AppError> {
    require_editor(&claims)?;
    let agent = state.db.create_agent(&req)?;
    let token = req.token.as_deref().unwrap_or("");
    let url = build_agent_url(&req.host, "/systeminfo")?;

    match fetch_agent_systeminfo(&url, token).await {
        Ok(info) => {
            let tracers = info.to_tracers();
            let tracers_json = serde_json::to_string(&tracers).unwrap_or_else(|_| "[]".to_string());
            let _ = state.db.update_agent_system_info(
                agent.id,
                &info.version,
                &tracers_json,
                "online",
                info.os.as_deref().unwrap_or(""),
                info.arch.as_deref().unwrap_or(""),
            );
            sync_tracepoints(&state.db, agent.id, &tracers);
            eprintln!(
                "[server] Agent {} synced: v{}, {} tracers",
                req.name,
                info.version,
                tracers.len()
            );
        }
        Err(e) => {
            eprintln!(
                "[server] Failed to fetch systeminfo from {}: {}",
                req.host, e
            );
        }
    }

    // Deploy selected sinks to the new agent
    if let Some(sink_ids) = &req.sink_ids {
        if !sink_ids.is_empty() {
            let mut deployed_sink_names = Vec::new();
            for &sink_id in sink_ids {
                let sink = match state.db.get_sink_by_id(sink_id) {
                    Ok(sink) => sink,
                    Err(err) => {
                        cleanup_created_agent(&state, agent.id, &deployed_sink_names).await;
                        return Err(err.into());
                    }
                };

                if let Err(err) =
                    create_agent_sink(&state, agent.id, &sink.name, &sink.sink_type, &sink.config)
                        .await
                {
                    cleanup_created_agent(&state, agent.id, &deployed_sink_names).await;
                    return Err(err);
                }

                if let Err(err) = state.db.bind_agent_to_sink(sink_id, agent.id) {
                    deployed_sink_names.push(sink.name);
                    cleanup_created_agent(&state, agent.id, &deployed_sink_names).await;
                    return Err(err.into());
                }
                deployed_sink_names.push(sink.name);
            }
        }
    }

    let agent = state.db.get_agent_by_id(agent.id)?;
    Ok(Json(agent))
}

/// Sync agent: re-fetch /systeminfo, update DB, sync tracepoints

async fn fetch_collector_status(
    host: &str,
    token: &str,
) -> Result<Vec<(String, String)>, AppError> {
    let url = build_agent_url(host, "/status")?;
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .map_err(|e| AppError::Internal(format!("HTTP client error: {}", e)))?;

    let mut request = client.get(&url);
    if !token.is_empty() {
        request = request.header("Authorization", format!("Bearer {}", token));
    }

    let response = request
        .send()
        .await
        .map_err(|e| AppError::Internal(format!("Failed to connect to agent: {}", e)))?;

    let status = response.status();
    let body = response.text().await.unwrap_or_default();

    if !status.is_success() {
        return Err(AppError::Internal(format!("Agent returned {}: {}", status, body)));
    }

    let v: serde_json::Value = serde_json::from_str(&body)
        .map_err(|e| AppError::Internal(format!("Invalid JSON: {}", e)))?;

    let items = v["collectors"]["items"]
        .as_array()
        .ok_or_else(|| AppError::Internal("Invalid collector status format".to_string()))?;

    let mut result = Vec::new();
    for item in items {
        if let (Some(name), Some(state)) = (
            item["name"].as_str(),
            item["state"].as_str(),
        ) {
            result.push((name.to_string(), state.to_string()));
        }
    }
    Ok(result)
}

pub async fn sync_agent(
    State(state): State<AppState>,
    claims: Claims,
    Path(id): Path<i64>,
) -> Result<Json<Agent>, AppError> {
    require_editor(&claims)?;
    let agent = state.db.get_agent_by_id(id)?;
    let token = agent.token.as_deref().unwrap_or("");
    let url = build_agent_url(&agent.host, "/systeminfo")?;

    eprintln!("[server] Syncing agent {} from {}", agent.name, url);

    match fetch_agent_systeminfo(&url, token).await {
        Ok(info) => {
            let tracers = info.to_tracers();
            let tracers_json = serde_json::to_string(&tracers).unwrap_or_else(|_| "[]".to_string());
            let _ = state.db.update_agent_system_info(
                id,
                &info.version,
                &tracers_json,
                "online",
                info.os.as_deref().unwrap_or(""),
                info.arch.as_deref().unwrap_or(""),
            );
            sync_tracepoints(&state.db, id, &tracers);

            // Fetch actual collector running state and update tracepoint enabled status
            match fetch_collector_status(&agent.host, token).await {
                Ok(collectors) => {
                    // First: set ALL tracepoints to stopped
                    let _ = state.db.set_all_tracepoints_enabled(id, false);
                    // Then: set running ones to true
                    for (name, state_str) in &collectors {
                        if state_str == "running" {
                            let _ = state.db.set_tracepoint_enabled(id, name, true);
                        }
                    }
                    eprintln!(
                        "[server] Synced agent {}: v{}, {} tracers, {} collectors ({} running)",
                        agent.name,
                        info.version,
                        tracers.len(),
                        collectors.len(),
                        collectors.iter().filter(|(_, s)| s == "running").count()
                    );
                }
                Err(e) => {
                    eprintln!("[server] Failed to fetch collector status: {}", e);
                }
            }
        }
        Err(e) => {
            eprintln!("[server] Sync failed for {}: {}", agent.name, e);
            let _ = state.db.update_agent_status(id, "offline");
            return Err(e);
        }
    }
    let agent = state.db.get_agent_by_id(id)?;
    Ok(Json(agent))
}

/// Start a tracer on agent
pub async fn start_tracer(
    State(state): State<AppState>,
    claims: Claims,
    Path((id, tracer_name)): Path<(i64, String)>,
) -> Result<Json<SuccessResponse>, AppError> {
    require_editor(&claims)?;
    state
        .db
        .get_tracepoint_by_agent_and_name(id, &tracer_name)?;
    start_agent_tracer(&state, id, &tracer_name).await?;
    if let Err(err) = state.db.enable_tracepoint_by_name(id, &tracer_name, true) {
        let _ = stop_agent_tracer(&state, id, &tracer_name).await;
        return Err(err.into());
    }
    Ok(Json(SuccessResponse { success: true }))
}

/// Stop a tracer on agent
pub async fn stop_tracer(
    State(state): State<AppState>,
    claims: Claims,
    Path((id, tracer_name)): Path<(i64, String)>,
) -> Result<Json<SuccessResponse>, AppError> {
    require_editor(&claims)?;
    state
        .db
        .get_tracepoint_by_agent_and_name(id, &tracer_name)?;
    stop_agent_tracer(&state, id, &tracer_name).await?;
    if let Err(err) = state.db.enable_tracepoint_by_name(id, &tracer_name, false) {
        let _ = start_agent_tracer(&state, id, &tracer_name).await;
        return Err(err.into());
    }
    Ok(Json(SuccessResponse { success: true }))
}

pub(crate) async fn start_agent_tracer(
    state: &AppState,
    agent_id: i64,
    tracer_name: &str,
) -> Result<(), AppError> {
    let sink_name = resolve_tracer_sink_name(state, agent_id, tracer_name);
    start_agent_tracer_with_sink_name(state, agent_id, tracer_name, sink_name).await
}

pub(crate) async fn start_agent_tracer_with_sink_name(
    state: &AppState,
    agent_id: i64,
    tracer_name: &str,
    sink_name: Option<String>,
) -> Result<(), AppError> {
    let agent = state.db.get_agent_by_id(agent_id)?;
    let token = agent.token.as_deref().unwrap_or("");
    let url = build_agent_url(&agent.host, &format!("/api/start/{}", tracer_name))?;

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| AppError::Internal(format!("Failed to create HTTP client: {}", e)))?;

    let mut request = client.post(&url);
    if !token.is_empty() {
        request = request.header("Authorization", format!("Bearer {}", token));
    }

    eprintln!("[start_tracer] sending sink_name={:?}", sink_name);
    request = request.json(&TracerStartRequest { sink_name });

    let response = request
        .send()
        .await
        .map_err(|e| AppError::Internal(format!("Failed to connect to agent: {}", e)))?;

    let status = response.status();
    if status.is_success() {
        return Ok(());
    }

    let body = response.text().await.unwrap_or_default();
    Err(AppError::Internal(format!(
        "Agent returned {}: {}",
        status, body
    )))
}

pub(crate) async fn stop_agent_tracer(
    state: &AppState,
    agent_id: i64,
    tracer_name: &str,
) -> Result<(), AppError> {
    let agent = state.db.get_agent_by_id(agent_id)?;
    let token = agent.token.as_deref().unwrap_or("");
    let url = build_agent_url(&agent.host, &format!("/api/stop/{}", tracer_name))?;

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| AppError::Internal(format!("Failed to create HTTP client: {}", e)))?;

    let mut request = client.post(&url);
    if !token.is_empty() {
        request = request.header("Authorization", format!("Bearer {}", token));
    }

    let response = request
        .send()
        .await
        .map_err(|e| AppError::Internal(format!("Failed to connect to agent: {}", e)))?;

    let status = response.status();
    let body = response.text().await.unwrap_or_default();

    if status.is_success() || (status == 404 && body.contains("stopped")) {
        return Ok(());
    }

    Err(AppError::Internal(format!(
        "Agent returned {}: {}",
        status, body
    )))
}

fn resolve_tracer_sink_name(state: &AppState, agent_id: i64, tracer_name: &str) -> Option<String> {
    match state
        .db
        .get_tracepoint_by_agent_and_name(agent_id, tracer_name)
    {
        Ok(tp) => {
            eprintln!(
                "[start_tracer] tracepoint={}, sink_id={:?}",
                tp.name, tp.sink_id
            );
            tp.sink_id
                .and_then(|sid| match state.db.get_sink_name_by_id(sid) {
                    Ok(name) => {
                        eprintln!("[start_tracer] resolved sink_name={:?}", name);
                        name
                    }
                    Err(e) => {
                        eprintln!("[start_tracer] get_sink_name_by_id error: {}", e);
                        None
                    }
                })
        }
        Err(e) => {
            eprintln!("[start_tracer] tracepoint lookup error: {}", e);
            None
        }
    }
}

async fn cleanup_created_agent(state: &AppState, agent_id: i64, sink_names: &[String]) {
    for sink_name in sink_names {
        let _ = rollback_agent_sinks(state, sink_name, &[agent_id]).await;
    }
    let _ = state.db.delete_agent(agent_id);
}

/// Sync tracepoints: create tracepoints for tracers that don't exist yet
fn sync_tracepoints(db: &Database, agent_id: i64, tracers: &[crate::models::agent::Tracer]) {
    let existing = db
        .list_all_tracepoints_for_agent(agent_id)
        .unwrap_or_default();
    let existing_names: std::collections::HashSet<String> =
        existing.into_iter().map(|tp| tp.name).collect();

    for tracer in tracers {
        if !existing_names.contains(&tracer.name) {
            let tp = CreateTracepoint {
                name: tracer.name.clone(),
                description: Some(tracer.description.clone()),
                enabled: Some(false),
                sink_id: None,
            };
            match db.create_tracepoint(agent_id, &tp) {
                Ok(_) => eprintln!(
                    "[server] Auto-created tracepoint: {} for agent {}",
                    tracer.name, agent_id
                ),
                Err(e) => eprintln!(
                    "[server] Failed to create tracepoint {}: {}",
                    tracer.name, e
                ),
            }
        }
    }
}

/// Fetch systeminfo from agent
async fn fetch_agent_systeminfo(url: &str, token: &str) -> Result<AgentSystemInfo, AppError> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .map_err(|e| AppError::Internal(format!("Failed to create HTTP client: {}", e)))?;

    let mut request = client.get(url);
    if !token.is_empty() {
        request = request.header("Authorization", format!("Bearer {}", token));
    }

    let response = request
        .send()
        .await
        .map_err(|e| AppError::Internal(format!("Failed to connect to agent: {}", e)))?;

    let status = response.status();
    let body = response.text().await.unwrap_or_default();

    decode_agent_systeminfo_response(status, &body)
}

fn decode_agent_systeminfo_response(
    status: StatusCode,
    body: &str,
) -> Result<AgentSystemInfo, AppError> {
    if !status.is_success() {
        return Err(agent_response_error(status, body));
    }

    serde_json::from_str(body).map_err(|e| {
        AppError::Internal(format!(
            "Failed to parse systeminfo: {} — body: {}",
            e,
            body.chars().take(300).collect::<String>()
        ))
    })
}

fn agent_response_error(status: StatusCode, body: &str) -> AppError {
    match status {
        StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => {
            AppError::BadRequest("Agent token is missing or invalid".to_string())
        }
        _ => AppError::Internal(format!(
            "Agent returned {}: {}",
            status,
            body.chars().take(200).collect::<String>()
        )),
    }
}

pub async fn update_agent(
    State(state): State<AppState>,
    claims: Claims,
    Path(id): Path<i64>,
    Json(req): Json<UpdateAgent>,
) -> Result<Json<Agent>, AppError> {
    require_editor(&claims)?;
    let agent = state.db.update_agent(id, &req)?;
    Ok(Json(agent))
}

pub async fn delete_agent(
    State(state): State<AppState>,
    claims: Claims,
    Path(id): Path<i64>,
) -> Result<Json<SuccessResponse>, AppError> {
    require_editor(&claims)?;
    let deleted = state.db.delete_agent(id)?;
    if deleted {
        Ok(Json(SuccessResponse { success: true }))
    } else {
        Err(AppError::NotFound("Agent not found".to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decode_agent_systeminfo_response_rejects_agent_auth_failure() {
        let err = decode_agent_systeminfo_response(StatusCode::UNAUTHORIZED, "bad token")
            .expect_err("agent auth failure should reject sync");

        assert!(matches!(
            err,
            AppError::BadRequest(message) if message == "Agent token is missing or invalid"
        ));
    }
}
