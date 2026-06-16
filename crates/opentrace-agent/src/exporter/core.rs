// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.
use std::future::Future;
use std::sync::Arc;
use std::time::Duration;

use opentrace_bpf::ProbeRegistry;
use prometheus::Registry;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

use crate::errors::AgntError;

pub(crate) struct ExporterContext {
    pub(crate) probe_registry: Arc<ProbeRegistry>,
    pub(crate) interval: Duration,
    pub(crate) cancel: CancellationToken,
}

pub(crate) trait ExporterRunner: Send + 'static {
    type Future: Future<Output = Result<(), AgntError>> + Send + 'static;

    fn run(self, context: ExporterContext) -> Self::Future;
}

impl<F, Fut> ExporterRunner for F
where
    F: FnOnce(ExporterContext) -> Fut + Send + 'static,
    Fut: Future<Output = Result<(), AgntError>> + Send + 'static,
{
    type Future = Fut;

    fn run(self, context: ExporterContext) -> Self::Future {
        self(context)
    }
}

pub(crate) struct ExporterSpec<R> {
    pub(crate) registry: Option<Registry>,
    pub(crate) sink_name: Option<String>,
    pub(crate) runner: R,
}

impl<R: ExporterRunner> ExporterSpec<R> {
    pub(crate) fn new(registry: Option<Registry>, sink_name: Option<String>, runner: R) -> Self {
        Self {
            registry,
            sink_name,
            runner,
        }
    }
}

pub struct Task {
    pub(crate) registry: Option<Registry>,
    cancel: CancellationToken,
    sink_name: Option<String>,
    pub handler: JoinHandle<Result<(), AgntError>>,
}
impl Task {
    pub(crate) fn new(
        registry: Option<Registry>,
        cancel: CancellationToken,
        handler: JoinHandle<Result<(), AgntError>>,
        sink_name: Option<String>,
    ) -> Self {
        Self {
            registry,
            cancel,
            sink_name,
            handler,
        }
    }

    pub(crate) fn stop(&self) {
        self.cancel.cancel();
    }

    pub(crate) fn sink_name(&self) -> Option<&str> {
        self.sink_name.as_deref()
    }
}
