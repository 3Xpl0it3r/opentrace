// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.

use std::sync::Arc;

use prometheus::Registry;
use serde::Deserialize;

use super::ApiResource;
use crate::errors::AgntError;
use crate::exporter::{SkbCollectorBuilder, SkbdropRequest};
use crate::manager::Manager;
use crate::sink::SinkConfig;

pub struct SkbdropResource;

impl ApiResource for SkbdropResource {
    type Request = SkbdropRequest;

    fn resource_name() -> &'static str {
        "skbdrop"
    }

    async fn start(manager: Arc<Manager>, req: Self::Request) -> Result<(), AgntError> {
        let sink_config: Option<SinkConfig> = if let Some(ref sink_name) = req.sink_name {
            Some(manager.get_sink(sink_name).await?)
        } else {
            None
        };

        let exporter = if let Some(cfg) = sink_config {
            match cfg {
                SinkConfig::Kafka(_) => SkbCollectorBuilder::prepare_kafka(req)?,
                SinkConfig::PrometheusPGW(_) => {
                    todo!("PrometheusPGW not yet implemented")
                }
            }
        } else {
            SkbCollectorBuilder::prepare_prometheus(req)?
        };
        manager.start("skbdrop", exporter).await?;
        Ok(())
    }

    async fn stop(manager: Arc<Manager>) -> Result<(), AgntError> {
        manager.stop("skbdrop").await?;
        Ok(())
    }
}
