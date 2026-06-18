use crate::agent_url::build_agent_url;
use crate::errors::AppError;
use crate::middleware::auth::{Claims, require_editor};
use crate::models::agent::SuccessResponse;
use crate::models::sink::{
    BindAgentRequest, CreateSink, CreateSinkResponse, Sink, SinkDeployResult, UpdateSink,
};
use crate::state::AppState;
use axum::{
    Json,
    extract::{Path, State},
};

pub async fn list_sinks(State(state): State<AppState>) -> Result<Json<Vec<Sink>>, AppError> {
    let sinks = state.db.list_sinks()?;
    Ok(Json(sinks))
}

pub async fn get_sink(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<Sink>, AppError> {
    let sink = state.db.get_sink_by_id(id)?;
    Ok(Json(sink))
}

pub async fn create_sink(
    State(state): State<AppState>,
    claims: Claims,
    Json(req): Json<CreateSink>,
) -> Result<Json<CreateSinkResponse>, AppError> {
    require_editor(&claims)?;

    let agent_ids = req.agent_ids.clone().unwrap_or_default();
    let deploy_results = if agent_ids.is_empty() {
        None
    } else {
        let results = deploy_sink_to_agents(&state, &req, &agent_ids).await;
        let success_count = results.iter().filter(|result| result.success).count();
        let fail_count = results.len() - success_count;
        eprintln!(
            "[server] Sink '{}' deployed: {} succeeded, {} failed",
            req.name, success_count, fail_count
        );

        if fail_count > 0 {
            let deployed_agent_ids = deployed_agent_ids(&results);
            rollback_agent_sinks(&state, &req.name, &deployed_agent_ids).await;
            return Err(AppError::Internal(format!(
                "sink deployment failed: {}",
                deploy_error_summary(&results)
            )));
        }

        Some(results)
    };

    let sink = match state.db.create_sink(&req) {
        Ok(sink) => sink,
        Err(err) => {
            rollback_agent_sinks(&state, &req.name, &agent_ids).await;
            return Err(err.into());
        }
    };
    for agent_id in &agent_ids {
        if let Err(err) = state.db.bind_agent_to_sink(sink.id, *agent_id) {
            rollback_agent_sinks(&state, &req.name, &agent_ids).await;
            let _ = state.db.delete_sink(sink.id);
            return Err(err.into());
        }
    }

    Ok(Json(CreateSinkResponse {
        id: sink.id,
        name: sink.name,
        sink_type: sink.sink_type,
        config: sink.config,
        status: sink.status,
        created_at: sink.created_at,
        deploy_results,
    }))
}

/// Deploy a sink to one or more agents by calling their API
async fn deploy_sink_to_agents(
    state: &AppState,
    sink: &CreateSink,
    agent_ids: &[i64],
) -> Vec<SinkDeployResult> {
    let mut results = Vec::new();

    for &agent_id in agent_ids {
        match create_agent_sink(state, agent_id, &sink.name, &sink.sink_type, &sink.config).await {
            Ok(agent_name) => results.push(SinkDeployResult {
                agent_id,
                agent_name,
                success: true,
                error: None,
            }),
            Err(e) => {
                let agent_name = state
                    .db
                    .get_agent_by_id(agent_id)
                    .map(|agent| agent.name)
                    .unwrap_or_else(|_| format!("unknown-{agent_id}"));
                results.push(SinkDeployResult {
                    agent_id,
                    agent_name,
                    success: false,
                    error: Some(e.to_string()),
                });
            }
        }
    }

    results
}

/// Convert server-side sink config to agent SinkConfig enum format.
/// Server stores flat JSON: {"brokers": [...], "topic": "..."}
/// Agent expects: {"config": {"Kafka": {"brokers": [...], "topic": "..."}}}
pub fn build_agent_sink_config(
    sink_type: &str,
    config: &str,
) -> Result<serde_json::Value, AppError> {
    let config_value: serde_json::Value = serde_json::from_str(config)
        .map_err(|e| AppError::BadRequest(format!("Invalid JSON config: {}", e)))?;

    let enum_value = match sink_type {
        "kafka" => serde_json::json!({ "Kafka": config_value }),
        "prometheus" => serde_json::json!({ "PrometheusPGW": config_value }),
        _ => {
            return Err(AppError::BadRequest(format!(
                "Unsupported sink type for agent deployment: {}",
                sink_type
            )));
        }
    };

    Ok(serde_json::json!({ "config": enum_value }))
}

pub(crate) async fn create_agent_sink(
    state: &AppState,
    agent_id: i64,
    sink_name: &str,
    sink_type: &str,
    config: &str,
) -> Result<String, AppError> {
    let agent = state.db.get_agent_by_id(agent_id)?;
    let url = build_agent_url(&agent.host, &format!("/api/sink/{sink_name}"))?;
    let agent_config = build_agent_sink_config(sink_type, config)?;
    send_agent_sink_request(
        &agent,
        reqwest::Method::PUT,
        &url,
        Some(&agent_config),
        false,
    )
    .await?;
    Ok(agent.name)
}

async fn update_agent_sink(
    state: &AppState,
    agent_id: i64,
    sink_name: &str,
    sink_type: &str,
    config: &str,
) -> Result<(), AppError> {
    let agent = state.db.get_agent_by_id(agent_id)?;
    let url = build_agent_url(&agent.host, &format!("/api/sink/{sink_name}"))?;
    let agent_config = build_agent_sink_config(sink_type, config)?;
    send_agent_sink_request(
        &agent,
        reqwest::Method::PATCH,
        &url,
        Some(&agent_config),
        false,
    )
    .await
}

pub(crate) async fn delete_agent_sink(
    state: &AppState,
    agent_id: i64,
    sink_name: &str,
) -> Result<(), AppError> {
    let agent = state.db.get_agent_by_id(agent_id)?;
    let url = build_agent_url(&agent.host, &format!("/api/sink/{sink_name}"))?;
    send_agent_sink_request(&agent, reqwest::Method::DELETE, &url, None, true).await
}

async fn send_agent_sink_request(
    agent: &crate::models::agent::Agent,
    method: reqwest::Method,
    url: &str,
    body: Option<&serde_json::Value>,
    not_found_is_success: bool,
) -> Result<(), AppError> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| AppError::Internal(format!("HTTP client error: {}", e)))?;

    let mut request = client.request(method, url);
    if let Some(token) = agent.token.as_deref().filter(|token| !token.is_empty()) {
        request = request.header("Authorization", format!("Bearer {token}"));
    }
    if let Some(body) = body {
        request = request.json(body);
    }

    let response = request
        .send()
        .await
        .map_err(|e| AppError::Internal(format!("Connection failed: {}", e)))?;
    let status = response.status();
    let body = response.text().await.unwrap_or_default();
    // 200 OK, 404 (not_found_is_success), 409 (already exists = already connected)
    if status.is_success() || (not_found_is_success && status.as_u16() == 404) || status.as_u16() == 409 {
        return Ok(());
    }

    Err(AppError::Internal(format!(
        "Agent returned {}: {}",
        status, body
    )))
}

fn deploy_error_summary(results: &[SinkDeployResult]) -> String {
    results
        .iter()
        .filter_map(|result| {
            result
                .error
                .as_ref()
                .map(|error| format!("{}: {}", result.agent_name, error))
        })
        .collect::<Vec<_>>()
        .join("; ")
}

fn deployed_agent_ids(results: &[SinkDeployResult]) -> Vec<i64> {
    results
        .iter()
        .filter(|result| result.success)
        .map(|result| result.agent_id)
        .collect()
}

pub(crate) async fn rollback_agent_sinks(state: &AppState, sink_name: &str, agent_ids: &[i64]) {
    for agent_id in agent_ids {
        let _ = delete_agent_sink(state, *agent_id, sink_name).await;
    }
}

pub async fn update_sink(
    State(state): State<AppState>,
    claims: Claims,
    Path(id): Path<i64>,
    Json(req): Json<UpdateSink>,
) -> Result<Json<Sink>, AppError> {
    require_editor(&claims)?;
    let current = state.db.get_sink_by_id(id)?;
    let agent_ids = state.db.get_sink_agents(id)?;
    let new_name = req.name.as_deref().unwrap_or(&current.name);
    let new_type = req.sink_type.as_deref().unwrap_or(&current.sink_type);
    let new_config = req.config.as_deref().unwrap_or(&current.config);

    if !agent_ids.is_empty() {
        if new_name != current.name {
            return Err(AppError::BadRequest(
                "cannot rename a sink while it is bound to agents".to_string(),
            ));
        }

        let mut updated_agents = Vec::new();
        for agent_id in &agent_ids {
            match update_agent_sink(&state, *agent_id, &current.name, new_type, new_config).await {
                Ok(()) => updated_agents.push(*agent_id),
                Err(err) => {
                    for rollback_agent_id in updated_agents {
                        let _ = update_agent_sink(
                            &state,
                            rollback_agent_id,
                            &current.name,
                            &current.sink_type,
                            &current.config,
                        )
                        .await;
                    }
                    return Err(err);
                }
            }
        }
    }

    let sink = match state.db.update_sink(id, &req) {
        Ok(sink) => sink,
        Err(err) => {
            for agent_id in &agent_ids {
                let _ = update_agent_sink(
                    &state,
                    *agent_id,
                    &current.name,
                    &current.sink_type,
                    &current.config,
                )
                .await;
            }
            return Err(err.into());
        }
    };
    Ok(Json(sink))
}

pub async fn delete_sink(
    State(state): State<AppState>,
    claims: Claims,
    Path(id): Path<i64>,
) -> Result<Json<SuccessResponse>, AppError> {
    require_editor(&claims)?;

    let sink = state.db.get_sink_by_id(id)?;
    let agent_ids = state.db.get_sink_agents(id)?;
    let mut deleted_agents = Vec::new();
    for agent_id in &agent_ids {
        if let Err(err) = delete_agent_sink(&state, *agent_id, &sink.name).await {
            for rollback_agent_id in deleted_agents {
                let _ = create_agent_sink(
                    &state,
                    rollback_agent_id,
                    &sink.name,
                    &sink.sink_type,
                    &sink.config,
                )
                .await;
            }
            return Err(err);
        }
        deleted_agents.push(*agent_id);
    }

    let deleted = match state.db.delete_sink(id) {
        Ok(deleted) => deleted,
        Err(err) => {
            for agent_id in &deleted_agents {
                let _ =
                    create_agent_sink(&state, *agent_id, &sink.name, &sink.sink_type, &sink.config)
                        .await;
            }
            return Err(err.into());
        }
    };
    if deleted {
        Ok(Json(SuccessResponse { success: true }))
    } else {
        for agent_id in &deleted_agents {
            let _ = create_agent_sink(&state, *agent_id, &sink.name, &sink.sink_type, &sink.config)
                .await;
        }
        Err(AppError::NotFound("Sink not found".to_string()))
    }
}

pub async fn bind_agent(
    State(state): State<AppState>,
    claims: Claims,
    Path(id): Path<i64>,
    Json(req): Json<BindAgentRequest>,
) -> Result<Json<SuccessResponse>, AppError> {
    require_editor(&claims)?;

    let sink = state.db.get_sink_by_id(id)?;
    create_agent_sink(
        &state,
        req.agent_id,
        &sink.name,
        &sink.sink_type,
        &sink.config,
    )
    .await?;
    if let Err(err) = state.db.bind_agent_to_sink(id, req.agent_id) {
        let _ = delete_agent_sink(&state, req.agent_id, &sink.name).await;
        return Err(err.into());
    }

    Ok(Json(SuccessResponse { success: true }))
}

pub async fn unbind_agent(
    State(state): State<AppState>,
    claims: Claims,
    Path((sink_id, agent_id)): Path<(i64, i64)>,
) -> Result<Json<SuccessResponse>, AppError> {
    require_editor(&claims)?;

    let sink = state.db.get_sink_by_id(sink_id)?;
    delete_agent_sink(&state, agent_id, &sink.name).await?;

    let deleted = match state.db.unbind_agent_from_sink(sink_id, agent_id) {
        Ok(deleted) => deleted,
        Err(err) => {
            let _ = create_agent_sink(&state, agent_id, &sink.name, &sink.sink_type, &sink.config)
                .await;
            return Err(err.into());
        }
    };
    if deleted {
        Ok(Json(SuccessResponse { success: true }))
    } else {
        let _ =
            create_agent_sink(&state, agent_id, &sink.name, &sink.sink_type, &sink.config).await;
        Err(AppError::NotFound("Binding not found".to_string()))
    }
}

/// Connect: send sink config to agent, agent creates and starts the sink
pub async fn connect_sink(
    State(state): State<AppState>,
    claims: Claims,
    Path((sink_id, agent_id)): Path<(i64, i64)>,
) -> Result<Json<SuccessResponse>, AppError> {
    require_editor(&claims)?;

    let sink = state.db.get_sink_by_id(sink_id)?;
    create_agent_sink(&state, agent_id, &sink.name, &sink.sink_type, &sink.config).await?;
    if let Err(err) = state.db.bind_agent_to_sink(sink_id, agent_id) {
        let _ = delete_agent_sink(&state, agent_id, &sink.name).await;
        return Err(err.into());
    }
    Ok(Json(SuccessResponse { success: true }))
}

/// Disconnect: tell agent to stop the sink (but keep the binding in DB)
pub async fn disconnect_sink(
    State(state): State<AppState>,
    claims: Claims,
    Path((sink_id, agent_id)): Path<(i64, i64)>,
) -> Result<Json<SuccessResponse>, AppError> {
    require_editor(&claims)?;

    let sink = state.db.get_sink_by_id(sink_id)?;
    delete_agent_sink(&state, agent_id, &sink.name).await?;
    Ok(Json(SuccessResponse { success: true }))
}

pub async fn get_sink_agents(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<Vec<i64>>, AppError> {
    let agents = state.db.get_sink_agents(id)?;
    Ok(Json(agents))
}

/// Get sinks deployed on a specific agent
pub async fn get_agent_sinks(
    State(state): State<AppState>,
    Path(agent_id): Path<i64>,
) -> Result<Json<Vec<String>>, AppError> {
    // Query the agent for its current sinks via its API
    let agent = state.db.get_agent_by_id(agent_id)?;
    let token = agent.token.as_deref().unwrap_or("");
    let url = build_agent_url(&agent.host, "/api/sinks")?;

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

    if response.status().is_success() {
        let body: serde_json::Value = response
            .json()
            .await
            .map_err(|e| AppError::Internal(format!("Invalid response: {}", e)))?;
        let sinks = body["sinks"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();
        Ok(Json(sinks))
    } else {
        Ok(Json(vec![]))
    }
}

/// Test sink connectivity by sending a test message via agent
pub async fn test_sink_connectivity(
    State(state): State<AppState>,
    claims: Claims,
    Path(sink_id): Path<i64>,
) -> Result<Json<serde_json::Value>, AppError> {
    require_editor(&claims)?;
    let sink = state.db.get_sink_by_id(sink_id)?;
    
    // Get all agents bound to this sink
    let agent_ids = state.db.get_sink_agents(sink_id)?;
    if agent_ids.is_empty() {
        return Err(AppError::BadRequest("Sink not bound to any agent".to_string()));
    }

    let config_value: serde_json::Value = serde_json::from_str(&sink.config)
        .map_err(|e| AppError::BadRequest(format!("Invalid config: {}", e)))?;

    // Build the agent sink config (tagged enum format)
    let agent_config = match sink.sink_type.as_str() {
        "kafka" => serde_json::json!({ "Kafka": config_value }),
        "prometheus" => serde_json::json!({ "PrometheusPGW": config_value }),
        _ => return Err(AppError::BadRequest(format!("Unsupported type: {}", sink.sink_type))),
    };

    let mut results = Vec::new();
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| AppError::Internal(format!("HTTP client error: {}", e)))?;

    for aid in &agent_ids {
        let agent = match state.db.get_agent_by_id(*aid) {
            Ok(a) => a,
            Err(_) => continue,
        };
        let token = agent.token.as_deref().unwrap_or("");
        let url = match build_agent_url(&agent.host, &format!("/api/sink/debug/{}", sink.sink_type)) {
            Ok(u) => {
                eprintln!("[test_sink] agent '{}' host='{}' url='{}'", agent.name, agent.host, u);
                u
            }
            Err(e) => {
                eprintln!("[test_sink] agent '{}' build_url error: {}", agent.name, e);
                results.push(serde_json::json!({"agent": agent.name, "success": false, "error": e.to_string()}));
                continue;
            }
        };

        let mut request = client.post(&url).json(&serde_json::json!({"config": agent_config}));
        if !token.is_empty() {
            request = request.header("Authorization", format!("Bearer {}", token));
        }

        match request.send().await {
            Ok(resp) if resp.status().is_success() => {
                results.push(serde_json::json!({"agent": agent.name, "success": true}));
            }
            Ok(resp) => {
                let status = resp.status();
                let body = resp.text().await.unwrap_or_default();
                results.push(serde_json::json!({"agent": agent.name, "success": false, "error": format!("{}: {}", status, body)}));
            }
            Err(e) => {
                results.push(serde_json::json!({"agent": agent.name, "success": false, "error": e.to_string()}));
            }
        }
    }

    let all_ok = results.iter().all(|r| r["success"] == true);
    if all_ok {
        Ok(Json(serde_json::json!({"success": true, "results": results})))
    } else {
        Err(AppError::Internal(serde_json::to_string(&serde_json::json!({"success": false, "results": results})).unwrap_or_default()))
    }
}
