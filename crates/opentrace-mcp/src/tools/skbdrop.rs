// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.

use std::sync::Arc;

use opentrace_bpf::sinks::UnboundedChannelSink;
use rmcp::model::{CallToolResult, Content};
use rmcp::{ErrorData, schemars};
use serde::Deserialize;
use tokio::time::Duration;
use tokio_util::sync::CancellationToken;

use opentrace_bpf::ProbeRegistry;
use opentrace_bpf::collectors::Collector;
use opentrace_bpf::collectors::net::{
    SkbdropCollector, SkbdropConfig, SkbdropEvent, SkbdropEventDefaultFormatter,
};
use opentrace_bpf::format::StreamFormatter;
use opentrace_bpf::protocols::{eth_proto, ip_proto};
use opentrace_bpf::symbolizers::{Source, SymbolizerProvider};

use crate::errors::MCPError;

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
    fn into_config(self) -> Result<SkbdropConfig, MCPError> {
        Ok(SkbdropConfig {
            any_addr: self.any_host.unwrap_or_default(),
            dst_addr: self.dst_host.unwrap_or_default(),
            src_addr: self.src_host.unwrap_or_default(),
            any_port: self.any_port.unwrap_or_default(),
            dst_port: self.dst_port.unwrap_or_default(),
            src_port: self.src_port.unwrap_or_default(),
            ip_proto: if let Some(ref proto) = self.ip_proto {
                ip_proto::parse(proto)?
            } else {
                ip_proto::TCP
            },
            eth_proto: if let Some(ref proto) = self.eth_proto {
                eth_proto::parse(proto)?
            } else {
                eth_proto::ETH_P_IP
            },
            ..Default::default()
        })
    }
}

struct CancelOnDrop(CancellationToken);

impl Drop for CancelOnDrop {
    fn drop(&mut self) {
        self.0.cancel();
    }
}

pub(crate) async fn tool_handler(
    params: SkbdropMcpToolParams,
    probe_registry: Arc<ProbeRegistry>,
) -> Result<CallToolResult, ErrorData> {
    let cancel = CancellationToken::new();
    let _cancel_on_drop = CancelOnDrop(cancel.clone());

    let handle = tokio::task::spawn_blocking(move || {
        run_skbdrop_blocking(params, probe_registry, Duration::from_secs(10 * 60), cancel)
            .map_err(|err| err.to_string())
    });

    let event = handle
        .await
        .map_err(|err| MCPError::Other(format!("skbdrop worker failed: {err}")))
        .and_then(|result| result.map_err(MCPError::Other))
        .map_err(ErrorData::from)?;

    Ok(match event {
        Some(event) => CallToolResult::success(vec![Content::text(event)]),
        None => CallToolResult::success(vec![]),
    })
}

fn run_skbdrop_blocking(
    params: SkbdropMcpToolParams,
    probe_registry: Arc<ProbeRegistry>,
    timeout: Duration,
    cancel: CancellationToken,
) -> Result<Option<String>, MCPError> {
    let mut open_project = opentrace_bpf::open_object_storage();
    let provider = SymbolizerProvider::default();
    let symbolizer = provider.get_symbolizer(&Source::Kernel);
    let formatter = SkbdropEventDefaultFormatter::new(symbolizer);

    let (sink, rx) = UnboundedChannelSink::<SkbdropEvent, SkbdropEvent>::new();
    let mut collector = SkbdropCollector::new(
        &mut open_project,
        probe_registry.as_ref(),
        params.into_config()?,
        sink,
    )
    .map_err(MCPError::from)?;
    collector.attach_probe().map_err(MCPError::from)?;

    //  等待10分钟，如果10分钟内抓不到包就退出
    receive_event_blocking(collector, rx, timeout, cancel, &formatter)
}

fn receive_event_blocking(
    mut collector: impl Collector,
    mut rx: tokio::sync::mpsc::UnboundedReceiver<SkbdropEvent>,
    timeout: Duration,
    cancel: CancellationToken,
    formatter: &impl StreamFormatter<SkbdropEvent>,
) -> Result<Option<String>, MCPError> {
    let deadline = std::time::Instant::now() + timeout;

    while std::time::Instant::now() < deadline && !cancel.is_cancelled() {
        if let Ok(event) = rx.try_recv() {
            let mut buf = Vec::new();
            formatter.format(&mut buf, &event).map_err(MCPError::from)?;
            let json_str = String::from_utf8(buf).map_err(MCPError::from)?;
            return Ok(Some(json_str));
        }

        collector.poll(Duration::from_millis(100))?;
    }

    Ok(None)
}
