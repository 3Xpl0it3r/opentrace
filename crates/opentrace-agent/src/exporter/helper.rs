// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.

use std::sync::Arc;
use std::time::Duration;

use opentrace_bpf::ProbeRegistry;
use prometheus::Registry;

use super::core::RunnerFuture;
use super::{Exporter, core::ExporterRunner};
use crate::errors::AgntError;

pub(crate) fn build_exporter(
    registry: Option<Registry>,
    runner: impl FnOnce(Arc<Exporter>, Arc<ProbeRegistry>, Duration) -> RunnerFuture + Send + 'static,
) -> Result<Arc<Exporter>, AgntError> {
    let exporter = Exporter::new(registry);
    let runner: ExporterRunner = Box::new(runner);
    exporter.set_runner(runner)?;
    Ok(Arc::new(exporter))
}
