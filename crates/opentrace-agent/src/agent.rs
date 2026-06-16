// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.

use std::sync::Arc;
use std::time::Duration;

use opentrace_bpf::ProbeRegistry;
use opentrace_kit::httpserver::GenericHttpServer;

use crate::errors::AgntError;
use crate::manager::Manager;

const METRICS_ENDPOINT: &str = "/metrics";

pub struct OpentraceAgent {
    manager: Arc<Manager>,
    server: GenericHttpServer,
}

impl OpentraceAgent {
    pub fn new(mut server: GenericHttpServer) -> Result<Self, AgntError> {
        let probe_registry = Arc::new(
            ProbeRegistry::try_init()
                .map_err(|e| AgntError::other(format!("init probe registry failed: {e}")))?,
        );
        let manager = Arc::new(Manager::new(probe_registry));

        // install metrics
        server
            .nest(METRICS_ENDPOINT, manager.metrics_router())
            .map_err(|e| AgntError::other(format!("install metrics apis failed {}", e)))?;

        server
            .nest("/api", crate::api::install_apis(manager.clone()))
            .map_err(|e| AgntError::other(format!("install apis failed {}", e)))?;

        Ok(Self { manager, server })
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
