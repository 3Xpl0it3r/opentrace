// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.

use std::sync::Arc;
use std::time::Duration;

use opentrace_bpf::ProbeRegistry;
use opentrace_kit::httpserver::GenericHttpServer;
use uuid::Uuid;

use crate::errors::AgntError;
use crate::manager::Manager;

const METRICS_ENDPOINT: &str = "/metrics";

pub struct OpentraceAgent {
    manager: Arc<Manager>,
    server: GenericHttpServer,
    #[allow(dead_code)]
    token: String,
}

impl OpentraceAgent {
    pub fn new(mut server: GenericHttpServer) -> Result<Self, AgntError> {
        let token = if server.has_auth() {
            String::new()
        } else {
            let token = Uuid::new_v4().to_string();
            eprintln!("[agent] API token: {token}");
            server.with_auth(token.clone());
            token
        };

        let probe_registry = Arc::new(
            ProbeRegistry::try_init()
                .map_err(|e| AgntError::other(format!("init probe registry failed: {e}")))?,
        );
        let manager = Arc::new(Manager::new(probe_registry));

        // install metrics
        server
            .nest_public(METRICS_ENDPOINT, manager.metrics_router())
            .map_err(|e| AgntError::other(format!("install metrics apis failed {}", e)))?;

        server
            .nest_auth(crate::systeminfo::ENDPOINT, crate::systeminfo::router())
            .map_err(|e| AgntError::other(format!("install systeminfo api failed {}", e)))?;

        server
            .nest(
                crate::manager::status::ENDPOINT,
                crate::manager::status::router(manager.clone()),
            )
            .map_err(|e| AgntError::other(format!("install status api failed {}", e)))?;

        server
            .nest_auth("/api", crate::api::install_apis(manager.clone()))
            .map_err(|e| AgntError::other(format!("install apis failed {}", e)))?;

        Ok(Self {
            manager,
            server,
            token,
        })
    }

    /// Returns the auto-generated bearer token.
    pub fn token(&self) -> &str {
        &self.token
    }

    pub async fn run(self) {
        if let Err(err) = self.server.run().await {
            eprintln!("server exited: {err}");
            _ = self.manager.stop_all().await;
        }
    }

    pub async fn stop(self) {
        _ = self.manager.stop_all().await;
        _ = self.manager.wait_terminated(Duration::from_secs(60)).await;
    }
}
