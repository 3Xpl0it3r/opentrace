// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.

use axum::{Router, http::StatusCode, routing::get};
use rmcp::transport::StreamableHttpService;

use crate::config::GenericConfig;

const HEALTH_ENDPOINT: &str = "/healthz";
const MCP_ENDPOINT: &str = "/mcp";

pub struct GenericServer<T> {
    cfg: GenericConfig,
    mcp: StreamableHttpService<T>,
}

impl<T: rmcp::ServerHandler> GenericServer<T> {
    pub fn new(cfg: GenericConfig, mcp: StreamableHttpService<T>) -> Self {
        Self { cfg, mcp }
    }

    pub async fn run(&self) {
        if !self.cfg.server_cert.is_empty() && !self.cfg.server_cert_key.is_empty() {
            self.run_https().await;
        } else {
            self.run_http().await;
        }
    }

    #[inline]
    pub async fn run_http(&self) {
        let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", self.cfg.bind_port))
            .await
            .unwrap();
        let router = Router::new()
            .route(HEALTH_ENDPOINT, get(health_handler))
            .nest_service(MCP_ENDPOINT, self.mcp.clone());

        axum::serve(listener, router).await;
    }

    #[inline]
    pub async fn run_https(&self) {}
}

async fn health_handler() -> StatusCode {
    StatusCode::OK
}
