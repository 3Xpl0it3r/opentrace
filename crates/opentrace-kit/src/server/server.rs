// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.

use std::sync::Arc;

use axum::middleware;
use axum::{Router, http::StatusCode, routing::get};
use axum_server::tls_rustls::RustlsConfig;

use super::authentication::AuthState;
use super::authentication::bearer_auth_middleware;
use super::config::{Config, SecurityConfig};
use super::errors::Error;

const HEALTH_ENDPOINT: &str = "/healthz";
/* const METRIC_ENDPOINT: &str = "/mtrics"; */

#[derive(Default)]
pub struct Server {
    cfg: Config,
    router: Router,
}

impl Server {
    pub fn new(cfg: Config) -> Self {
        let mut router = Router::new();

        if !cfg.bear_token.is_empty() {
            let auth_state = AuthState {
                bearer_token: Arc::from(cfg.bear_token.clone()),
            };
            router = Router::new().layer(middleware::from_fn_with_state(
                auth_state.clone(),
                bearer_auth_middleware,
            ));
        }

        Self { cfg, router }
    }

    pub async fn run(self) -> Result<(), Error> {
        if self.cfg.is_tls() {
            self.serve_tls().await
        } else {
            self.serve().await;
            Ok(())
        }
    }

    // 是否开启 heath
    pub fn enable_health(&mut self) {
        self.router = self
            .router
            .clone()
            .route(HEALTH_ENDPOINT, get(health_handler))
    }

    pub fn nest(&mut self, path: &str, router: Router) -> Result<(), Error> {
        self.router = self.router.clone().nest(path, router);
        Ok(())
    }

    #[inline]
    pub async fn serve(self) {
        let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", self.cfg.bind_port))
            .await
            .unwrap();

        let _ = axum::serve(listener, self.router).await;
    }

    #[inline]
    pub async fn serve_tls(self) -> Result<(), Error> {
        let addr: std::net::SocketAddr = format!("0.0.0.0:{}", self.cfg.bind_port)
            .parse()
            .map_err(|e| Error::Other(format!("parse address failed: {}", e)))?;
        let tls_config = build_tls_config((&self.cfg).into()).await?;
        axum_server::bind_rustls(addr, tls_config)
            .serve(self.router.into_make_service())
            .await
            .map_err(|e| Error::Other(format!("bind tls failed {}", e)))?;
        Ok(())
    }
}

async fn health_handler() -> StatusCode {
    StatusCode::OK
}

async fn build_tls_config(security_config: SecurityConfig) -> Result<RustlsConfig, Error> {
    RustlsConfig::from_pem_file(
        &security_config.server_cert,
        &security_config.server_cert_key,
    )
    .await
    .map_err(|e| Error::Other(format!("faile build tls config {}", e)))
}
