// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use opentrace_bpf::ProbeRegistry;
use opentrace_bpf::collectors::Collector;
use prometheus::Registry;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

use crate::errors::AgntError;

const STATE_NEW: u8 = 0;
const STATE_RUNNING: u8 = 1;
const STATE_STOPPING: u8 = 2;
const STATE_TERMINATED: u8 = 3;
const STATE_FAILED: u8 = 4;

pub struct Task {
    pub exporter: Arc<Exporter>,
    pub handler: JoinHandle<Result<(), AgntError>>,
}

pub(super) type RunnerFuture =
    Pin<Box<dyn Future<Output = Result<(), AgntError>> + Send + 'static>>;
pub(super) type ExporterRunner =
    Box<dyn FnOnce(Arc<Exporter>, Arc<ProbeRegistry>, Duration) -> RunnerFuture + Send + 'static>;

pub struct Exporter {
    pub state: AtomicU8,
    cancel: CancellationToken,
    pub registry: Option<Registry>,
    runner: Mutex<Option<ExporterRunner>>,
}

impl Exporter {
    pub fn new(registry: Option<Registry>) -> Self {
        Self {
            runner: Mutex::new(None),
            state: AtomicU8::new(STATE_NEW),
            cancel: CancellationToken::new(),
            registry,
        }
    }

    pub fn set_runner(&self, runner: ExporterRunner) -> Result<(), AgntError> {
        if self.state.load(Ordering::Acquire) != STATE_NEW {
            return Err(AgntError::Other("exporter already started".into()));
        }
        *self.runner.lock().expect("runner mutex poisoned") = Some(runner);
        Ok(())
    }

    pub fn stop(&self) {
        self.state.store(STATE_STOPPING, Ordering::Release);
        self.cancel.cancel();
    }

    pub fn start(
        self: &Arc<Self>,
        interval: Duration,
        probe_registry: Arc<ProbeRegistry>,
    ) -> Result<JoinHandle<Result<(), AgntError>>, AgntError> {
        let runner = self
            .runner
            .lock()
            .map_err(|e| AgntError::other(format!("runner mutex poisoned: {e}")))?
            .take()
            .ok_or_else(|| AgntError::other("runner not set"))?;

        self.state.store(STATE_RUNNING, Ordering::Release);
        let runner_exporter = Arc::clone(self);
        let state_exporter = Arc::clone(self);

        Ok(tokio::task::spawn(async move {
            let result = runner(runner_exporter, probe_registry, interval).await;
            let final_state = match &result {
                Ok(()) => STATE_TERMINATED,
                Err(_) => STATE_FAILED,
            };
            state_exporter.state.store(final_state, Ordering::Release);
            result
        }))
    }
}

pub(super) async fn run(
    exporter: &Exporter,
    collector: &mut dyn Collector,
    probe_registry: Arc<ProbeRegistry>,
    interval: Duration,
) -> Result<(), AgntError> {
    collector
        .attach_probe(&probe_registry)
        .map_err(AgntError::other)?;

    while !exporter.cancel.is_cancelled() {
        collector.poll(interval).map_err(AgntError::other)?;
        tokio::task::yield_now().await;
    }

    Ok(())
}
