use crate::agent_url::build_agent_url;
use crate::errors::AppError;
use crate::middleware::auth::{Claims, require_editor};
use crate::models::agent::SuccessResponse;
use crate::state::AppState;
use axum::Json;
use axum::extract::{Path, Query, State};
use axum::response::sse::{Event, KeepAlive, Sse};
use futures::stream::Stream;
use serde::Deserialize;
use std::convert::Infallible;
use std::time::Duration;
use tokio::sync::mpsc;

const SSE_CHANNEL_CAPACITY: usize = 256;
const KEEP_ALIVE_INTERVAL: Duration = Duration::from_secs(15);

#[derive(Debug, Deserialize)]
pub struct DebugQuery {
    pub tracer: String,
    pub param: Option<String>,
}

/// SSE proxy: connects to agent's watch endpoint and streams events to the frontend
pub async fn watch(
    State(state): State<AppState>,
    claims: Claims,
    Path(id): Path<i64>,
    Query(query): Query<DebugQuery>,
) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, AppError> {
    require_editor(&claims)?;
    let agent = state.db.get_agent_by_id(id)?;
    let token = agent.token.as_deref().unwrap_or("");

    // Build request body with watch: true
    let mut body = serde_json::json!({ "watch": true });
    if let Some(ref p) = query.param {
        if !p.is_empty() {
            if let Ok(filter) = serde_json::from_str::<serde_json::Value>(p) {
                if let Some(obj) = filter.as_object() {
                    for (k, v) in obj {
                        body.as_object_mut().unwrap().insert(k.clone(), v.clone());
                    }
                }
            } else {
                body.as_object_mut()
                    .unwrap()
                    .insert("src_addr".into(), serde_json::Value::String(p.clone()));
            }
        }
    }

    let url = build_agent_url(&agent.host, &format!("/api/start/{}", query.tracer))?;
    let tracer_name = query.tracer.clone();
    eprintln!("[debug] watch: connecting to {}", url);

    // Use connect_timeout only — no overall timeout for SSE streams
    let client = reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(10))
        .build()
        .map_err(|e| AppError::Internal(format!("Failed to create HTTP client: {}", e)))?;

    let mut request = client.post(&url).json(&body);
    if !token.is_empty() {
        request = request.header("Authorization", format!("Bearer {}", token));
    }

    let response = request.send().await.map_err(|e| {
        eprintln!("[debug] watch: connection failed: {}", e);
        AppError::Internal(format!("Failed to connect to agent: {}", e))
    })?;

    let status = response.status();
    eprintln!("[debug] watch: agent responded with status {}", status);

    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        eprintln!("[debug] watch: agent error: {}", body);
        return Err(AppError::Internal(format!(
            "Agent returned {}: {}",
            status, body
        )));
    }

    let byte_stream = response.bytes_stream();
    let (tx, rx) = mpsc::channel::<Result<Event, Infallible>>(SSE_CHANNEL_CAPACITY);
    state.db.enable_tracepoint_by_name(id, &tracer_name, true)?;
    let cleanup_db = state.db.clone();

    // Spawn task to parse SSE from agent and forward
    tokio::spawn(async move {
        use futures::StreamExt;
        let mut buffer = String::new();
        let mut event_type = String::new();
        let mut event_data = String::new();
        let mut event_count: u64 = 0;
        let mut client_disconnected = false;

        let mut stream = byte_stream;
        while let Some(chunk_result) = stream.next().await {
            let chunk = match chunk_result {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("[debug] watch: stream error: {}", e);
                    break;
                }
            };

            buffer.push_str(&String::from_utf8_lossy(&chunk));

            while let Some(newline_pos) = buffer.find('\n') {
                let line = buffer[..newline_pos].trim_end_matches('\r').to_string();
                buffer = buffer[newline_pos + 1..].to_string();

                if line.is_empty() {
                    // Empty line = dispatch event
                    if !event_data.is_empty() {
                        event_count += 1;
                        let mut event = Event::default().data(&event_data);
                        if !event_type.is_empty() {
                            event = event.event(&event_type);
                        }
                        if tx.send(Ok(event)).await.is_err() {
                            eprintln!(
                                "[debug] watch: client disconnected after {} events",
                                event_count
                            );
                            client_disconnected = true;
                            break;
                        }
                    }
                    event_type.clear();
                    event_data.clear();
                } else if let Some(t) = line.strip_prefix("event:") {
                    event_type = t.trim().to_string();
                } else if let Some(d) = line.strip_prefix("data:") {
                    if !event_data.is_empty() {
                        event_data.push('\n');
                    }
                    event_data.push_str(d.trim());
                } else if line.starts_with(':') {
                    // Comment / keep-alive, ignore
                }
            }

            if client_disconnected {
                break;
            }
        }

        // Flush remaining
        if !client_disconnected && !event_data.is_empty() {
            let mut event = Event::default().data(&event_data);
            if !event_type.is_empty() {
                event = event.event(&event_type);
            }
            let _ = tx.send(Ok(event)).await;
        }
        let _ = cleanup_db.enable_tracepoint_by_name(id, &tracer_name, false);
        eprintln!("[debug] watch: stream ended, total events: {}", event_count);
    });

    let stream = tokio_stream::wrappers::ReceiverStream::new(rx);
    Ok(Sse::new(stream).keep_alive(KeepAlive::new().interval(KEEP_ALIVE_INTERVAL)))
}

/// Stop debug session
pub async fn stop(
    State(state): State<AppState>,
    claims: Claims,
    Path(id): Path<i64>,
    Query(query): Query<DebugQuery>,
) -> Result<Json<SuccessResponse>, AppError> {
    require_editor(&claims)?;
    let agent = state.db.get_agent_by_id(id)?;
    let token = agent.token.as_deref().unwrap_or("");

    let url = build_agent_url(&agent.host, &format!("/api/stop/{}", query.tracer))?;
    eprintln!("[debug] stop: sending stop to {}", url);

    let client = reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(10))
        .build()
        .map_err(|e| AppError::Internal(format!("Failed to create HTTP client: {}", e)))?;

    eprintln!(
        "[debug] stop: url={}, token={}",
        url,
        if token.is_empty() { "empty" } else { "set" }
    );
    let mut request = client.post(&url);
    if !token.is_empty() {
        request = request.header("Authorization", format!("Bearer {}", token));
    }

    let response = request.send().await.map_err(|e| {
        eprintln!("[debug] stop: request failed: {}", e);
        AppError::Internal(format!("Failed to connect to agent: {}", e))
    })?;

    let status = response.status();
    let body = response.text().await.unwrap_or_default();
    eprintln!("[debug] stop: agent responded {} {}", status, body);

    if status.is_success() || (status == 404 && body.contains("stopped")) {
        state
            .db
            .enable_tracepoint_by_name(id, &query.tracer, false)?;
        return Ok(Json(SuccessResponse { success: true }));
    }

    Err(AppError::Internal(format!(
        "Agent returned {}: {}",
        status, body
    )))
}
