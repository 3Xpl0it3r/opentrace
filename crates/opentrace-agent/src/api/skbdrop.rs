// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.

use std::sync::Arc;

use super::ApiResource;
use crate::errors::AgntError;
use crate::exporter::{SkbdropExporter, SkbdropRequest};
use crate::manager::Manager;

pub struct SkbdropResource;

impl ApiResource for SkbdropResource {
    type Request = SkbdropRequest;

    fn resource_name() -> &'static str {
        "skbdrop"
    }

    async fn start(manager: Arc<Manager>, req: Self::Request) -> Result<(), AgntError> {
        if let Some(sink_name) = req.sink_name.clone() {
            let sink_tx = manager.get_sink(&sink_name).await?;
            let exporter = SkbdropExporter::with_sink(req, sink_tx, sink_name)?;
            manager.start("skbdrop", exporter).await?;
        } else {
            let exporter = SkbdropExporter::with_prometheus_metrics(req)?;
            manager.start("skbdrop", exporter).await?;
        }
        Ok(())
    }

    async fn stop(manager: Arc<Manager>) -> Result<(), AgntError> {
        manager.stop("skbdrop").await?;
        Ok(())
    }
}
