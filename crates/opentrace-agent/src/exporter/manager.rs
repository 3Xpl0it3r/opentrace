// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use opentrace_bpf::ProbeRegistry;
use prometheus::{Encoder, TextEncoder};
use tokio::sync::RwLock;
use tokio_util::sync::CancellationToken;

use crate::errors::AgntError;

use super::{ExporterContext, ExporterRunner, ExporterSpec, ExporterTask};

pub(crate) struct ExporterManager {
    tasks: RwLock<HashMap<String, ExporterTask>>,
}

impl ExporterManager {
    pub(crate) fn new() -> Self {
        Self {
            tasks: RwLock::new(HashMap::default()),
        }
    }

    pub(crate) async fn start<R>(
        &self,
        name: &str,
        exporter: ExporterSpec<R>,
        probe_registry: Arc<ProbeRegistry>,
        poll_interval: Duration,
    ) -> Result<(), AgntError>
    where
        R: ExporterRunner,
    {
        let mut tasks = self.tasks.write().await;
        if tasks.contains_key(name) {
            return Err(AgntError::AlreadyExists(format!("{} 已经启动了", name)));
        }

        let ExporterSpec {
            registry,
            sink_name,
            runner,
        } = exporter;
        let cancel = CancellationToken::new();
        let context = ExporterContext {
            probe_registry,
            interval: poll_interval,
            cancel: cancel.clone(),
        };
        let handler = tokio::spawn(runner.run(context));

        tasks.insert(
            name.to_owned(),
            ExporterTask::new(registry, cancel, handler, sink_name),
        );
        Ok(())
    }

    pub(crate) async fn stop_all(&self) {
        for task in self.tasks.read().await.values() {
            task.stop();
        }
    }

    pub(crate) async fn stop(&self, name: &str, deadline: Instant) -> Result<(), AgntError> {
        let task = self
            .tasks
            .write()
            .await
            .remove(name)
            .ok_or_else(|| AgntError::NotFound(format!("{} has stopped", name)))?;
        task.stop();
        Self::wait_task_until(task, deadline).await;
        Ok(())
    }

    pub(crate) async fn wait_terminated(&self, deadline: Instant) {
        let tasks: Vec<_> = {
            let mut tasks = self.tasks.write().await;
            tasks.drain().map(|(_, task)| task).collect()
        };

        for task in tasks {
            task.stop();
            Self::wait_task_until(task, deadline).await;
        }
    }

    pub(crate) async fn sink_is_used(&self, name: &str) -> bool {
        self.tasks
            .read()
            .await
            .values()
            .any(|task| task.sink_name() == Some(name))
    }

    pub(crate) async fn encode_all<W: std::io::Write>(
        &self,
        w: &mut W,
    ) -> Result<(), prometheus::Error> {
        let encoder = TextEncoder::new();
        for task in self.tasks.read().await.values() {
            if let Some(ref registry) = task.registry {
                encoder.encode(&registry.gather(), w)?;
            }
        }
        Ok(())
    }

    async fn wait_task_until(mut task: ExporterTask, deadline: Instant) {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            task.handler.abort();
            _ = task.handler.await;
            return;
        }

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
