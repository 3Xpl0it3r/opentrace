// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.

use std::sync::Arc;

use crate::db;

#[derive(Clone)]
pub struct AppState {
    pub db: Arc<db::Database>,
    pub jwt_secret: Arc<str>,
}

impl AppState {
    pub fn new(db: Arc<db::Database>, jwt_secret: String) -> Self {
        Self {
            db,
            jwt_secret: Arc::from(jwt_secret),
        }
    }
}
