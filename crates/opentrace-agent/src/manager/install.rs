// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.

use std::sync::Arc;

use axum::Router;
use axum::routing::{delete, get, put};

use crate::api::{ApiRouter, SkbdropResource, add_sink, list_sinks, remove_sink, update_sink};

use super::Manager;

pub fn install_apis(manager: Arc<Manager>) -> Router {
    // sink相关的api
    let sink_router = Router::new()
        .route("/sink/{name}", put(add_sink))
        .route("/sink/{name}", delete(remove_sink))
        .route("/sink/{name}", get(list_sinks))
        .route("/sinks", get(list_sinks))
        .with_state(manager.clone());

    // bpf resoruce 相关的接口
    Router::new()
        .with_resource::<SkbdropResource>(manager)
        .merge(sink_router)
}
