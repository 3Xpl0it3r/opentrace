// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.

use std::sync::Arc;

use axum::Json;
use axum::Router;
use axum::extract::State;
use axum::routing::get;
use serde::Serialize;

use crate::manager::Manager;

pub(crate) const ENDPOINT: &str = "/status";

#[derive(Serialize)]
struct CollectorStatus {
    name: String,
    state: &'static str,
    sink_name: Option<String>,
}

#[derive(Serialize)]
struct CollectorStatusSummary {
    total: usize,
    running: usize,
    stopped: usize,
    items: Vec<CollectorStatus>,
}

#[derive(Serialize)]
struct StatusResponse {
    collectors: CollectorStatusSummary,
}

pub(crate) fn router(manager: Arc<Manager>) -> Router {
    Router::new()
        .route("/", get(status_handler))
        .with_state(manager)
}

async fn status_handler(State(manager): State<Arc<Manager>>) -> Json<StatusResponse> {
    let collectors: Vec<_> = manager
        .collector_status()
        .await
        .into_iter()
        .map(|(name, state, sink_name)| CollectorStatus {
            name,
            state,
            sink_name,
        })
        .collect();
    let running = collectors
        .iter()
        .filter(|collector| collector.state == "running")
        .count();
    let total = collectors.len();
    let stopped = total - running;

    Json(StatusResponse {
        collectors: CollectorStatusSummary {
            total,
            running,
            stopped,
            items: collectors,
        },
    })
}
