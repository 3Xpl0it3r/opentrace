// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.

use std::sync::Arc;

use axum::extract::{Json, Path, State};
use axum::http::StatusCode;
use serde::{Deserialize, Serialize};

use crate::errors::AgntError;
use crate::manager::Manager;
use crate::sink::{KafkaSink, SinkConfig};

#[derive(Deserialize)]
pub struct SinkRequest {
    pub config: SinkConfig,
}

#[derive(Serialize)]
pub struct SinkListResponse {
    pub sinks: Vec<String>,
}

pub async fn add_sink_handler(
    State(manager): State<Arc<Manager>>,
    Path(name): Path<String>,
    Json(req): Json<SinkRequest>,
) -> Result<StatusCode, AgntError> {
    manager.add_sink(name, req.config).await?;
    Ok(StatusCode::CREATED)
}

pub async fn update_sink_handler(
    State(manager): State<Arc<Manager>>,
    Path(name): Path<String>,
    Json(req): Json<SinkRequest>,
) -> Result<StatusCode, AgntError> {
    manager.update_sink(&name, req.config).await?;
    Ok(StatusCode::OK)
}

pub async fn debug_sink_handler(
    Path(sink_type): Path<String>,
    Json(req): Json<SinkRequest>,
) -> Result<StatusCode, AgntError> {
    match (sink_type.to_ascii_lowercase().as_str(), req.config) {
        ("kafka", SinkConfig::Kafka(kafka_config)) => {
            KafkaSink::send_debug(kafka_config)?;
            Ok(StatusCode::OK)
        }
        ("kafka", _) => Err(AgntError::BadRequest(
            "sink debug type 'kafka' requires Kafka config".to_string(),
        )),
        (sink_type, _) => Err(AgntError::BadRequest(format!(
            "sink debug type '{sink_type}' is not supported"
        ))),
    }
}

pub async fn remove_sink_handler(
    State(manager): State<Arc<Manager>>,
    Path(name): Path<String>,
) -> Result<StatusCode, AgntError> {
    manager.remove_sink(&name).await?;
    Ok(StatusCode::OK)
}

pub async fn list_sinks_handler(
    State(manager): State<Arc<Manager>>,
) -> Result<Json<SinkListResponse>, AgntError> {
    let sinks = manager.list_sinks().await;
    Ok(Json(SinkListResponse { sinks }))
}
