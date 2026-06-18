// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.

use std::env;

pub struct Config {
    pub database_path: String,
    pub jwt_secret: String,
    pub port: u16,
}

impl Default for Config {
    fn default() -> Self {
        Self::from_env()
    }
}

impl Config {
    pub fn from_env() -> Self {
        Self {
            database_path: env::var("DATABASE_PATH").unwrap_or_else(|_| "opentrace.db".to_string()),
            jwt_secret: env::var("JWT_SECRET").unwrap_or_default(),
            port: env::var("PORT")
                .unwrap_or_else(|_| "8000".to_string())
                .parse()
                .unwrap_or(8000),
        }
    }
}
