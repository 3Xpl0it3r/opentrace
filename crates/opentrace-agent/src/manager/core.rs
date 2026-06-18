// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.

use std::sync::Arc;
use std::time::{Duration, Instant};

use axum::Router;
use axum::extract::State;
use axum::http::{StatusCode, header};
use axum::response::IntoResponse;
use axum::routing::get;
use prometheus::{Encoder, TextEncoder};

use opentrace_bpf::ProbeRegistry;

use crate::errors::AgntError;
use crate::exporter::{ExporterManager, ExporterRunner, ExporterSpec};
use crate::sink::{SinkConfig, SinkManager, SinkRecordSender};

const DEFAULT_POLL_INTERVAL: Duration = Duration::from_millis(100);
const DEFAULT_STOP_TIMEOUT: Duration = Duration::from_secs(60);

pub struct Manager {
    exporter_manager: ExporterManager,
    probe_registry: Arc<ProbeRegistry>,
    sink_manager: SinkManager,
}

impl Manager {
    pub fn new(probe_registry: Arc<ProbeRegistry>) -> Self {
        Self {
            exporter_manager: ExporterManager::new(),
            sink_manager: SinkManager::new(),
            probe_registry,
        }
    }

    // start 去创建一个具体的exporter(也就是collector)，然后去spawn一个async函数在里面attach和poll
    pub async fn start<R>(&self, name: &str, exporter: ExporterSpec<R>) -> Result<(), AgntError>
    where
        R: ExporterRunner,
    {
        self.exporter_manager
            .start(
                name,
                exporter,
                self.probe_registry.clone(),
                DEFAULT_POLL_INTERVAL,
            )
            .await
    }

    pub async fn stop_all(&self) -> Result<(), AgntError> {
        self.exporter_manager.stop_all().await;
        self.sink_manager.stop_all().await;
        Ok(())
    }

    pub async fn stop(&self, name: &str) -> Result<(), AgntError> {
        self.exporter_manager
            .stop(name, Instant::now() + DEFAULT_STOP_TIMEOUT)
            .await
    }

    pub async fn wait_terminated(&self, timeout: Duration) -> Result<(), AgntError> {
        let deadline = Instant::now() + timeout;
        self.exporter_manager.wait_terminated(deadline).await;
        self.sink_manager.wait_terminated(deadline).await;
        Ok(())
    }

    pub fn metrics_router(self: &Arc<Self>) -> Router {
        Router::<Arc<Self>>::new()
            .route("/", get(Self::metrics_handler))
            .with_state(Arc::clone(self))
    }

    async fn metrics_handler(State(manager): State<Arc<Self>>) -> impl IntoResponse {
        let mut buf = Vec::new();
        if let Err(err) = manager.encode_all(&mut buf).await {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("encode error: {err}"),
            )
                .into_response();
        }
        (
            [(header::CONTENT_TYPE, TextEncoder::new().format_type())],
            buf,
        )
            .into_response()
    }

    pub async fn get_sink(&self, sink_name: &str) -> Result<SinkRecordSender, AgntError> {
        self.sink_manager.get_sink(sink_name).await
    }

    pub async fn add_sink(&self, name: String, config: SinkConfig) -> Result<(), AgntError> {
        self.sink_manager.add_sink(name, config).await
    }

    pub async fn update_sink(&self, name: &str, config: SinkConfig) -> Result<(), AgntError> {
        if self.sink_is_used(name).await {
            return Err(AgntError::AlreadyExists(format!(
                "sink '{name}' is in use by exporter"
            )));
        }

        self.sink_manager
            .update_sink(name, config, Instant::now() + DEFAULT_STOP_TIMEOUT)
            .await
    }

    pub async fn remove_sink(&self, name: &str) -> Result<(), AgntError> {
        if self.sink_is_used(name).await {
            return Err(AgntError::AlreadyExists(format!(
                "sink '{name}' is in use by exporter"
            )));
        }

        self.sink_manager
            .remove_sink(name, Instant::now() + DEFAULT_STOP_TIMEOUT)
            .await
    }

    pub async fn list_sinks(&self) -> Vec<String> {
        self.sink_manager.list_sinks().await
    }

    pub(crate) async fn collector_status(&self) -> Vec<(String, &'static str, Option<String>)> {
        self.exporter_manager.status().await
    }

    async fn sink_is_used(&self, name: &str) -> bool {
        self.exporter_manager.sink_is_used(name).await
    }

    async fn encode_all<W: std::io::Write>(&self, w: &mut W) -> Result<(), prometheus::Error> {
        self.exporter_manager.encode_all(w).await
    }
}
