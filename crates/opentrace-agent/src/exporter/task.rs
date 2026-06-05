// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.
use std::sync::{Arc, Mutex};
use std::time::Duration;

use opentrace_bpf::ProbeRegistry;
use prometheus::Registry;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

use opentrace_bpf::collectors::Collector;

use crate::errors::AgntError;

use super::CollectorBuildeFn;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum State {
    New,
    Running,
    Stopping,
    Terminated,
    Failed,
}

pub struct Task {
    pub exporter: Arc<Exporter>,
    pub handler: JoinHandle<Result<(), AgntError>>,
}

pub struct Exporter {
    pub state: Mutex<State>,
    cancel: CancellationToken,
    pub registry: Registry,
    builder: Mutex<Option<Box<CollectorBuildeFn>>>,
}

impl Exporter {
    pub fn new(registry: Registry) -> Self {
        Self {
            builder: Mutex::new(None),
            state: Mutex::new(State::New),
            cancel: CancellationToken::new(),
            registry,
        }
    }

    pub fn set_builder(&self, builder: Box<CollectorBuildeFn>) {
        *self.builder.lock().expect("builder mutex poisoned") = Some(builder);
    }

    pub fn stop(&self) {
        *self.state.lock().expect("state mutex poisoned") = State::Stopping;
        self.cancel.cancel();
    }

    pub fn start(
        self: &Arc<Self>,
        interval: Duration,
        probe_registry: Arc<ProbeRegistry>,
    ) -> Result<JoinHandle<Result<(), AgntError>>, AgntError> {
        let builder = self
            .builder
            .lock()
            .map_err(|e| AgntError::Other(format!("builder mutex poisoned: {e}")))?
            .take()
            .ok_or(AgntError::Other("builder not set".into()))?;
        *self
            .state
            .lock()
            .map_err(|e| AgntError::Other(format!("state mutex poisoned: {e}")))? = State::Running;
        let exporter = Arc::clone(self);

        Ok(tokio::task::spawn(async move {
            let object = opentrace_bpf::open_object_storage();
            let (_object, mut collector) =
                builder(object).map_err(|e| AgntError::Other(e.to_string()))?;

            let result = run(&exporter, &mut *collector, probe_registry, interval).await;
            let final_state = match &result {
                Ok(()) => State::Terminated,
                Err(_) => State::Failed,
            };
            *exporter.state.lock().expect("state mutex poisoned") = final_state;
            result
        }))
    }
}

async fn run(
    exporter: &Exporter,
    collector: &mut (dyn Collector + 'static),
    probe_registry: Arc<ProbeRegistry>,
    interval: Duration,
) -> Result<(), AgntError> {
    collector
        .attach_probe(&probe_registry)
        .map_err(|e| AgntError::Other(e.to_string()))?;

    while !exporter.cancel.is_cancelled() {
        collector
            .poll(interval)
            .map_err(|e| AgntError::Other(e.to_string()))?;
        tokio::task::yield_now().await;
    }

    Ok(())
}
