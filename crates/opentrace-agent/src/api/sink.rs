// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.

use std::sync::Arc;

use axum::extract::{Json, Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use serde::{Deserialize, Serialize};

use crate::errors::AgntError;
use crate::manager::Manager;
use crate::sink::SinkConfig;

#[derive(Deserialize)]
pub struct SinkRequest {
    pub config: SinkConfig,
}

#[derive(Serialize)]
pub struct SinkListResponse {
    pub sinks: Vec<String>,
}

pub async fn add_sink(
    State(manager): State<Arc<Manager>>,
    Path(name): Path<String>,
    Json(req): Json<SinkRequest>,
) -> Result<StatusCode, AgntError> {
    manager.add_sink(name, req.config).await?;
    Ok(StatusCode::CREATED)
}

pub async fn update_sink(
    State(manager): State<Arc<Manager>>,
    Path(name): Path<String>,
    Json(req): Json<SinkRequest>,
) -> Result<StatusCode, AgntError> {
    manager.update_sink(&name, req.config).await?;
    Ok(StatusCode::OK)
}

pub async fn remove_sink(
    State(manager): State<Arc<Manager>>,
    Path(name): Path<String>,
) -> Result<StatusCode, AgntError> {
    manager.remove_sink(&name).await?;
    Ok(StatusCode::OK)
}

pub async fn list_sinks(
    State(manager): State<Arc<Manager>>,
) -> Result<Json<SinkListResponse>, AgntError> {
    let sinks = manager.list_sinks().await;
    Ok(Json(SinkListResponse { sinks }))
}
