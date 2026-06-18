// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.

use std::sync::Arc;

use axum::Router;
use axum::routing::{delete, get, patch, post, put};

use crate::manager::Manager;

mod skbdrop;
mod resource;
mod sink;

pub use resource::{ApiResource, ApiRouter};
pub use sink::{
    add_sink_handler, debug_sink_handler, list_sinks_handler, remove_sink_handler,
    update_sink_handler,
};
pub use skbdrop::SkbdropResource;

pub fn install_apis(manager: Arc<Manager>) -> Router {
    // sink相关的api
    let sink_router = Router::new()
        .route("/sink/debug/{sink_type}", post(debug_sink_handler))
        .route("/sink/{name}", put(add_sink_handler))
        .route("/sink/{name}", patch(update_sink_handler))
        .route("/sink/{name}", delete(remove_sink_handler))
        .route("/sink/{name}", get(list_sinks_handler))
        .route("/sinks", get(list_sinks_handler))
        .with_state(manager.clone());

    // bpf resoruce 相关的接口
    Router::new()
        .with_resource::<SkbdropResource>(manager)
        .merge(sink_router)
}
