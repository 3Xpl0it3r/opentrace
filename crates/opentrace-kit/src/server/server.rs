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
    public_router: Router,
    auth_router: Router,
    auth_state: Option<AuthState>,
}

impl Server {
    pub fn new(cfg: Config) -> Self {
        let bear_token = cfg.bear_token.clone();
        let mut server = Self {
            cfg,
            router: Router::new(),
            public_router: Router::new(),
            auth_router: Router::new(),
            auth_state: None,
        };

        if !bear_token.is_empty() {
            server.auth_state = Some(AuthState {
                bearer_token: bear_token.into(),
            })
        }

        server
    }

    pub async fn run(self) -> Result<(), Error> {
        let mut server = self;
        server.build_router();

        if server.cfg.is_tls() {
            server.serve_tls().await
        } else {
            server.serve().await;
            Ok(())
        }
    }

    // 是否开启 heath
    pub fn enable_health(&mut self) {
        let router = std::mem::take(&mut self.router);
        self.router = router.route(HEALTH_ENDPOINT, get(health_handler))
    }

    pub fn nest(&mut self, path: &str, router: Router) -> Result<(), Error> {
        nest_into(&mut self.router, path, router);
        Ok(())
    }

    /// Nest a router that never uses bearer auth.
    pub fn nest_public(&mut self, path: &str, router: Router) -> Result<(), Error> {
        nest_into(&mut self.public_router, path, router);
        Ok(())
    }

    /// Nest a router that uses bearer auth when auth is configured.
    pub fn nest_auth(&mut self, path: &str, router: Router) -> Result<(), Error> {
        nest_into(&mut self.auth_router, path, router);
        Ok(())
    }

    // 如果手动指定了with_auth, 则覆盖config里面指定的bearauth
    pub fn with_auth(&mut self, bear_token: impl Into<Arc<str>>) {
        let bearer_token = bear_token.into();
        if bearer_token.is_empty() {
            self.auth_state = None;
        } else {
            self.auth_state = Some(AuthState { bearer_token });
        }
    }

    pub fn has_auth(&self) -> bool {
        self.auth_state.is_some()
    }

    #[inline]
    async fn serve(self) {
        let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", self.cfg.bind_port))
            .await
            .unwrap();

        let _ = axum::serve(listener, self.router).await;
    }

    #[inline]
    async fn serve_tls(self) -> Result<(), Error> {
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

    fn build_router(&mut self) {
        let public_router = std::mem::take(&mut self.public_router);
        let default_router = std::mem::take(&mut self.router);
        let auth_router = std::mem::take(&mut self.auth_router);
        let protected_router = default_router.merge(auth_router);

        if let Some(auth_state) = self.auth_state.clone() {
            self.router = public_router.merge(protected_router.layer(
                middleware::from_fn_with_state(auth_state, bearer_auth_middleware),
            ));
        } else {
            self.router = public_router.merge(protected_router);
        }
    }
}

fn nest_into(target: &mut Router, path: &str, router: Router) {
    let target_router = std::mem::take(target);
    *target = target_router.nest(path, router);
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

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, header};
    use tower::ServiceExt;

    async fn ok_handler() -> StatusCode {
        StatusCode::OK
    }

    fn test_router() -> Router {
        Router::new().route("/ok", get(ok_handler))
    }

    async fn request(router: Router, path: &str, token: Option<&str>) -> StatusCode {
        let mut request = Request::builder()
            .uri(path)
            .body(Body::empty())
            .expect("request");
        if let Some(token) = token {
            request.headers_mut().insert(
                header::AUTHORIZATION,
                format!("Bearer {token}")
                    .parse()
                    .expect("authorization header"),
            );
        }

        router.oneshot(request).await.expect("response").status()
    }

    #[test]
    fn public_routes_bypass_auth() {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("runtime");

        runtime.block_on(async {
            let mut server = Server::new(Config {
                bear_token: "token".to_string(),
                ..Config::default()
            });
            server
                .nest_public("/public", test_router())
                .expect("public route");
            server
                .nest_auth("/private", test_router())
                .expect("auth route");
            server.build_router();

            assert_eq!(
                request(server.router.clone(), "/public/ok", None).await,
                StatusCode::OK
            );
            assert_eq!(
                request(server.router.clone(), "/private/ok", None).await,
                StatusCode::UNAUTHORIZED
            );
            assert_eq!(
                request(server.router, "/private/ok", Some("token")).await,
                StatusCode::OK
            );
        });
    }

    #[test]
    fn default_routes_keep_global_auth_behavior() {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("runtime");

        runtime.block_on(async {
            let mut server = Server::new(Config {
                bear_token: "token".to_string(),
                ..Config::default()
            });
            server
                .nest("/default", test_router())
                .expect("default route");
            server.build_router();

            assert_eq!(
                request(server.router.clone(), "/default/ok", None).await,
                StatusCode::UNAUTHORIZED
            );
            assert_eq!(
                request(server.router, "/default/ok", Some("token")).await,
                StatusCode::OK
            );
        });
    }

    #[test]
    fn auth_routes_are_public_without_auth_state() {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("runtime");

        runtime.block_on(async {
            let mut server = Server::default();
            server
                .nest_auth("/private", test_router())
                .expect("auth route");
            server.build_router();

            assert_eq!(
                request(server.router, "/private/ok", None).await,
                StatusCode::OK
            );
        });
    }
}
