// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.

use std::collections::HashMap;
use std::time::Instant;

use serde::{Deserialize, Serialize};
use tokio::sync::{RwLock, mpsc};
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

use crate::errors::AgntError;

use super::kafka::{KafkaConfig, KafkaRecord, KafkaSink};
use super::prometheus::{PrometheusConfig, PrometheusRecord, PrometheusSink};

#[derive(Clone)]
pub enum SinkRecordSender {
    Kafka(mpsc::Sender<KafkaRecord>),
    PrometheusPushGateway(mpsc::Sender<PrometheusRecord>),
}
pub enum SinkRecordReceiver {
    Kafka(mpsc::Receiver<KafkaRecord>),
    PrometheusPGW(mpsc::Receiver<PrometheusRecord>),
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum SinkConfig {
    Kafka(KafkaConfig),
    PrometheusPGW(PrometheusConfig),
}

pub(crate) struct SinkManager {
    registry: RwLock<HashMap<String, SinkTask>>,
}

struct SinkTask {
    config: SinkConfig,
    tx: SinkRecordSender,
    cancel: CancellationToken,
    handler: JoinHandle<Result<(), AgntError>>,
}

impl SinkManager {
    pub(crate) fn new() -> Self {
        Self {
            registry: RwLock::new(HashMap::default()),
        }
    }

    pub(crate) async fn get_sink(&self, sink_name: &str) -> Result<SinkRecordSender, AgntError> {
        self.registry
            .read()
            .await
            .get(sink_name)
            .map(|sink| sink.tx.clone())
            .ok_or_else(|| AgntError::NotFound(format!("sink '{sink_name}' not found")))
    }

    pub(crate) async fn add_sink(&self, name: String, config: SinkConfig) -> Result<(), AgntError> {
        let mut registry = self.registry.write().await;
        if registry.contains_key(&name) {
            return Err(AgntError::AlreadyExists(format!(
                "sink '{name}' already exists"
            )));
        }

        registry.insert(name, SinkTask::run(config));
        Ok(())
    }

    pub(crate) async fn update_sink(
        &self,
        name: &str,
        config: SinkConfig,
        deadline: Instant,
    ) -> Result<(), AgntError> {
        let runtime = self
            .registry
            .write()
            .await
            .remove(name)
            .ok_or_else(|| AgntError::NotFound(format!("sink '{name}' not found")))?;
        Self::wait_sink_until(runtime, deadline).await;

        self.registry
            .write()
            .await
            .insert(name.to_owned(), SinkTask::run(config));
        Ok(())
    }

    pub(crate) async fn remove_sink(&self, name: &str, deadline: Instant) -> Result<(), AgntError> {
        let runtime = self
            .registry
            .write()
            .await
            .remove(name)
            .ok_or_else(|| AgntError::NotFound(format!("sink '{name}' not found")))?;
        runtime.cancel.cancel();
        Self::wait_sink_until(runtime, deadline).await;
        Ok(())
    }

    pub(crate) async fn list_sinks(&self) -> Vec<String> {
        self.registry.read().await.keys().cloned().collect()
    }

    pub(crate) async fn stop_all(&self) {
        for sink in self.registry.read().await.values() {
            sink.cancel.cancel();
        }
    }

    pub(crate) async fn wait_terminated(&self, deadline: Instant) {
        let runtimes: Vec<_> = {
            let mut sinks = self.registry.write().await;
            sinks.drain().map(|(_, runtime)| runtime).collect()
        };

        for runtime in runtimes {
            Self::wait_sink_until(runtime, deadline).await;
        }
    }

    async fn wait_sink_until(runtime: SinkTask, deadline: Instant) {
        let SinkTask {
            config: _config,
            tx,
            cancel,
            mut handler,
        } = runtime;

        cancel.cancel();
        drop(tx);

        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            handler.abort();
            _ = handler.await;
            return;
        }

        let sleep = tokio::time::sleep(remaining);
        tokio::pin!(sleep);
        tokio::select! {
            _ = &mut handler => {}
            _ = &mut sleep => {
                handler.abort();
                _ = handler.await;
            }
        }
    }
}

impl SinkTask {
    fn run(config: SinkConfig) -> Self {
        let cancel = CancellationToken::new();
        let (tx, handler) = match &config {
            SinkConfig::Kafka(kafka_config) => {
                let (tx, rx) = mpsc::channel::<KafkaRecord>(1024);
                let handler =
                    tokio::spawn(KafkaSink::new(kafka_config.clone()).run(rx, cancel.clone()));
                (SinkRecordSender::Kafka(tx), handler)
            }
            SinkConfig::PrometheusPGW(prometheus_config) => {
                let (tx, rx) = mpsc::channel::<PrometheusRecord>(1024);
                let handler = tokio::spawn(
                    PrometheusSink::new(prometheus_config.clone()).run(rx, cancel.clone()),
                );
                (SinkRecordSender::PrometheusPushGateway(tx), handler)
            }
        };
        Self {
            config,
            tx,
            cancel,
            handler,
        }
    }
}
