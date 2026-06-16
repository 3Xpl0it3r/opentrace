// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.

use std::sync::Arc;
use std::time::Duration;

use opentrace_bpf::ProbeRegistry;
use opentrace_bpf::collectors::Collector;
use tokio_util::sync::CancellationToken;

use crate::errors::AgntError;

pub(crate) async fn run_collector(
    collector: &mut dyn Collector,
    probe_registry: Arc<ProbeRegistry>,
    interval: Duration,
    cancel: CancellationToken,
) -> Result<(), AgntError> {
    if cancel.is_cancelled() {
        return Ok(());
    }

    collector
        .attach_probe(&probe_registry)
        .map_err(AgntError::other)?;

    while !cancel.is_cancelled() {
        collector.poll(interval).map_err(AgntError::other)?;
        tokio::task::yield_now().await;
    }

    Ok(())
}
