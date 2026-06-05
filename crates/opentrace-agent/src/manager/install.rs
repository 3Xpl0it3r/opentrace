// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.

use std::sync::Arc;

use axum::Router;

use crate::api::{ApiRouter, SkbdropResource};

use super::Manager;

pub fn install_apis(manager: Arc<Manager>) -> Router {
    Router::new().with_resource::<SkbdropResource>(manager)
}
