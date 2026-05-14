// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.

use std::mem::MaybeUninit;

use rmcp::model::CallToolResult;
use rmcp::{ErrorData, schemars};
use serde::Deserialize;
use tokio::time::Duration;

use opentrace_bpf::ProbeRegistry;
use opentrace_bpf::collector::Collector;
use opentrace_bpf::collector::net::{SkbdropCollector, SkbdropConfig, SkbdropEvent};
use opentrace_bpf::format::JsonFormatter;
use opentrace_bpf::protocols::{eth_proto, ip_proto};

use crate::errors::MCPError;
use crate::exporter::{McpExporter, receive_event_sync};

// Parameters accepted by the skbdrop MCP tool.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub(crate) struct SkbdropMcpToolParams {
    #[schemars(
        description = "IP address to filter (matches if either source or destination address matches)"
    )]
    #[serde(skip_serializing_if = "Option::is_none")]
    any_host: Option<String>,

    #[schemars(description = "Destination IP address to filter")]
    #[serde(skip_serializing_if = "Option::is_none")]
    dst_host: Option<String>,

    #[schemars(description = "Source IP address to filter")]
    #[serde(skip_serializing_if = "Option::is_none")]
    src_host: Option<String>,

    #[schemars(
        description = "Port to filter (matches if either source or destination port matches, e.g. 80, 443)"
    )]
    #[serde(skip_serializing_if = "Option::is_none")]
    any_port: Option<u16>,

    #[schemars(description = "Destination port to filter (e.g. 80, 443)")]
    #[serde(skip_serializing_if = "Option::is_none")]
    dst_port: Option<u16>,

    #[schemars(description = "Source port to filter (e.g. 8080)")]
    #[serde(skip_serializing_if = "Option::is_none")]
    src_port: Option<u16>,

    #[schemars(description = "IP protocol number (6=TCP, 17=UDP, 1=ICMP)")]
    ip_proto: Option<String>,

    #[schemars(description = "Ethernet frame protocol (0x0800=IPv4, 0x86DD=IPv6)")]
    eth_proto: Option<String>,
}

impl SkbdropMcpToolParams {
    fn to_config(self) -> Result<SkbdropConfig, MCPError> {
        let mut config = SkbdropConfig::default();

        config.any_addr = self.any_host.unwrap_or_default();
        config.dst_addr = self.dst_host.unwrap_or_default();
        config.src_addr = self.src_host.unwrap_or_default();

        config.any_port = self.any_port.unwrap_or_default();
        config.dst_port = self.dst_port.unwrap_or_default();
        config.src_port = self.src_port.unwrap_or_default();

        if let Some(ref proto) = self.ip_proto {
            config.ip_proto = ip_proto::parse(proto)?;
        } else {
            config.ip_proto = ip_proto::TCP;
        }
        if let Some(ref proto) = self.eth_proto {
            config.eth_proto = eth_proto::parse(proto)?;
        } else {
            config.eth_proto = eth_proto::ETH_P_IP;
        }

        Ok(config)
    }
}

pub(crate) fn tool_handler(
    params: SkbdropMcpToolParams,
    probe_registry: &ProbeRegistry,
) -> Result<CallToolResult, ErrorData> {
    let mut open_project = opentrace_bpf::open_object_storage();
    let (exporter, rx) = McpExporter::new(
        10,
        JsonFormatter::default(),
        opentrace_bpf::symbol::new_kernel_symbol(),
    );

    let mut collector = SkbdropCollector::new(
        &mut open_project,
        probe_registry,
        params.to_config().map_err(MCPError::from)?,
        exporter,
    )
    .unwrap();
    collector.attach_probe().unwrap();

    //  等待10分钟，如果10分钟内抓不到包就退出
    receive_event_sync(collector, rx, Duration::from_mins(10), JsonFormatter).map_err(|e| e.into())
}
