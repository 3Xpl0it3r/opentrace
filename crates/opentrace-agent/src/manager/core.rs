// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use axum::Router;
use axum::extract::State;
use axum::http::{StatusCode, header};
use axum::response::IntoResponse;
use axum::routing::get;
use prometheus::{Encoder, TextEncoder};
use tokio::sync::RwLock;

use opentrace_bpf::ProbeRegistry;

use crate::errors::AgntError;
use crate::exporter::{Exporter, ExporterTask};
use crate::sink::SinkConfig;

const DEFAULT_POLL_INTERVAL: Duration = Duration::from_millis(100);

pub struct Manager {
    exporter_tasks: RwLock<HashMap<String, ExporterTask>>,
    probe_registry: Arc<ProbeRegistry>,
    // sink配置信息, 下沉到每个exporter去创建
    sink_registry: RwLock<HashMap<String, SinkConfig>>,
}

impl Manager {
    pub fn new(probe_registry: Arc<ProbeRegistry>) -> Self {
        Self {
            exporter_tasks: RwLock::new(HashMap::default()),
            sink_registry: RwLock::new(HashMap::default()),
            probe_registry,
        }
    }

    // start 去创建一个具体的exporter(也就是collector)，然后去spawn一个async函数在里面attach和poll
    pub async fn start(&self, name: &str, exporter: Arc<Exporter>) -> Result<(), AgntError> {
        if self.exporter_tasks.read().await.contains_key(name) {
            return Err(AgntError::AlreadyExists(format!("{} 已经启动了", name)));
        }
        let probe_registry = self.probe_registry.clone();
        let jh = exporter.start(DEFAULT_POLL_INTERVAL, probe_registry)?;

        self.exporter_tasks.write().await.insert(
            name.to_owned(),
            ExporterTask {
                exporter,
                handler: jh,
            },
        );
        Ok(())
    }

    pub async fn stop_all(&self) -> Result<(), AgntError> {
        for task in self.exporter_tasks.read().await.values() {
            task.exporter.stop();
        }
        self.exporter_tasks.write().await.clear();
        Ok(())
    }

    pub async fn stop(&self, name: &str) -> Result<(), AgntError> {
        let task = self.exporter_tasks.read().await.get(name).map(|t| {
            t.exporter.stop();
        });
        if task.is_none() {
            return Err(AgntError::NotFound(format!("{} has stopped", name)));
        }
        self.exporter_tasks.write().await.remove(name);
        Ok(())
    }

    pub async fn wait_terminated(&self, timeout: Duration) -> Result<(), AgntError> {
        let exporter_tasks: Vec<_> = {
            let mut tasks = self.exporter_tasks.write().await;
            tasks.drain().map(|(_, h)| h).collect()
        };

        if exporter_tasks.is_empty() {
            return Ok(());
        }

        let deadline = Instant::now() + timeout;
        for mut task in exporter_tasks {
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                task.handler.abort();
                _ = task.handler.await;
            } else {
                let sleep = tokio::time::sleep(remaining);
                tokio::pin!(sleep);
                tokio::select! {
                    _ = &mut task.handler => {}
                    _ = &mut sleep => {
                        task.handler.abort();
                        _ = task.handler.await;
                    }
                }
            }
        }
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

    pub async fn get_sink(&self, sink_name: &str) -> Result<SinkConfig, AgntError> {
        self.sink_registry
            .read()
            .await
            .get(sink_name)
            .cloned()
            .ok_or_else(|| AgntError::NotFound(format!("sink '{sink_name}' not found")))
    }

    pub async fn add_sink(&self, name: String, config: SinkConfig) -> Result<(), AgntError> {
        let mut registry = self.sink_registry.write().await;
        if registry.contains_key(&name) {
            return Err(AgntError::AlreadyExists(format!(
                "sink '{name}' already exists"
            )));
        }
        registry.insert(name, config);
        Ok(())
    }

    pub async fn update_sink(&self, name: &str, config: SinkConfig) -> Result<(), AgntError> {
        let mut registry = self.sink_registry.write().await;
        if !registry.contains_key(name) {
            return Err(AgntError::NotFound(format!("sink '{name}' not found")));
        }
        registry.insert(name.to_owned(), config);
        Ok(())
    }

    pub async fn remove_sink(&self, name: &str) -> Result<(), AgntError> {
        let mut registry = self.sink_registry.write().await;
        registry
            .remove(name)
            .ok_or_else(|| AgntError::NotFound(format!("sink '{name}' not found")))?;
        Ok(())
    }

    pub async fn list_sinks(&self) -> Vec<String> {
        self.sink_registry.read().await.keys().cloned().collect()
    }

    async fn encode_all<W: std::io::Write>(&self, w: &mut W) -> Result<(), prometheus::Error> {
        let encoder = TextEncoder::new();
        for task in self.exporter_tasks.read().await.values() {
            if let Some(ref registry) = task.exporter.registry {
                encoder.encode(&registry.gather(), w)?;
            }
        }
        Ok(())
    }
}
