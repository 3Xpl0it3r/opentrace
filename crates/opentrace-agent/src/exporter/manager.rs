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
    stopping: RwLock<HashMap<String, Option<String>>>,
    stopped: RwLock<HashMap<String, Option<String>>>,
}

impl ExporterManager {
    pub(crate) fn new() -> Self {
        Self {
            tasks: RwLock::new(HashMap::default()),
            stopping: RwLock::new(HashMap::default()),
            stopped: RwLock::new(HashMap::default()),
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
        let stopping = self.stopping.read().await;
        if stopping.contains_key(name) {
            return Err(AgntError::AlreadyExists(format!(
                "{} 正在暂停，请稍后重试",
                name
            )));
        }

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
        drop(stopping);
        self.stopped.write().await.remove(name);
        Ok(())
    }

    pub(crate) async fn stop_all(&self) {
        for task in self.tasks.read().await.values() {
            task.stop();
        }
    }

    pub(crate) async fn stop(&self, name: &str, deadline: Instant) -> Result<(), AgntError> {
        let mut stopping = self.stopping.write().await;
        if stopping.contains_key(name) {
            return Err(AgntError::AlreadyExists(format!("{} is stopping", name)));
        }

        let task = {
            let mut tasks = self.tasks.write().await;
            tasks
                .remove(name)
                .ok_or_else(|| AgntError::NotFound(format!("{} has stopped", name)))?
        };
        let sink_name = task.sink_name().map(str::to_owned);
        stopping.insert(name.to_owned(), sink_name.clone());
        drop(stopping);

        task.stop();
        Self::wait_task_until(task, deadline).await;
        self.stopping.write().await.remove(name);
        self.stopped
            .write()
            .await
            .insert(name.to_owned(), sink_name);
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
        let is_running = self
            .tasks
            .read()
            .await
            .values()
            .any(|task| task.sink_name() == Some(name));
        let is_stopping = self
            .stopping
            .read()
            .await
            .values()
            .any(|sink_name| sink_name.as_deref() == Some(name));
        is_running || is_stopping
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

    pub(crate) async fn status(&self) -> Vec<(String, &'static str, Option<String>)> {
        let mut status: Vec<_> = self
            .stopped
            .read()
            .await
            .iter()
            .map(|(name, sink_name)| (name.clone(), "stopped", sink_name.clone()))
            .collect();

        status.extend(
            self.stopping
                .read()
                .await
                .iter()
                .map(|(name, sink_name)| (name.clone(), "stopping", sink_name.clone())),
        );

        status.extend(self.tasks.read().await.iter().map(|(name, task)| {
            let state = if task.is_running() {
                "running"
            } else {
                "stopped"
            };
            (name.clone(), state, task.sink_name().map(str::to_owned))
        }));

        status
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
