// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.

use std::sync::Arc;

use opentrace_bpf::collectors::net::SkbdropConfig;
use serde::Deserialize;

use super::ApiResource;
use crate::errors::AgntError;
use crate::exporter::SkbCollectorBuilder;
use crate::manager::Manager;

#[derive(Debug, Deserialize)]
pub struct SkbdropRequest {
    pub any_addr: Option<String>,
    pub src_addr: Option<String>,
    pub dst_addr: Option<String>,
    pub any_port: Option<u16>,
    pub src_port: Option<u16>,
    pub dst_port: Option<u16>,
}

impl From<SkbdropRequest> for SkbdropConfig {
    fn from(value: SkbdropRequest) -> Self {
        Self {
            any_addr: value.any_addr.unwrap_or_default(),
            src_addr: value.src_addr.unwrap_or_default(),
            dst_addr: value.dst_addr.unwrap_or_default(),
            any_port: value.any_port.unwrap_or_default(),
            src_port: value.src_port.unwrap_or_default(),
            dst_port: value.dst_port.unwrap_or_default(),
            ..Default::default()
        }
    }
}

pub struct SkbdropResource;

impl ApiResource for SkbdropResource {
    type Request = SkbdropRequest;

    fn path_prefix() -> &'static str {
        "skbdrop"
    }

    async fn start(manager: Arc<Manager>, req: Self::Request) -> Result<(), AgntError> {
        let config: SkbdropConfig = req.into();
        let exporter = SkbCollectorBuilder::prepare(config)?;
        manager.start("skbdrop", exporter).await?;
        Ok(())
    }

    async fn stop(manager: Arc<Manager>, name: String) -> Result<(), AgntError> {
        manager.stop(&name).await?;
        Ok(())
    }
}
