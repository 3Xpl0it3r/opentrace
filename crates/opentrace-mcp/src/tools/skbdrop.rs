// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.

use std::io::{self, Write};
use std::marker::PhantomData;
use std::mem;
use std::sync::Arc;

use rmcp::model::{CallToolResult, Content};
use rmcp::{ErrorData, schemars};
use serde::ser::Error as _;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::{Receiver, Sender, channel};
use tokio::time::Duration;
use tokio_util::sync::CancellationToken;

use opentrace_bpf::collector::Collector;
use opentrace_bpf::collector::net::{SkbdropCollector, SkbdropConfig, SkbdropEvent};
use opentrace_bpf::format::StreamFormatter;
use opentrace_bpf::protocols::{eth_proto, ip_proto};
use opentrace_bpf::symbol::{Source, SymbolizeInput, Symbolizer, SymbolizerProvider};
use opentrace_bpf::{Exporter, ProbeRegistry};

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

struct SymRef<'a>(&'a dyn Symbolizer);

impl Symbolizer for SymRef<'_> {
    fn resolve(&self, input: SymbolizeInput) -> opentrace_bpf::symbol::ResolvedSymbol<'_> {
        self.0.resolve(input)
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
    let symbolizer = SymRef(provider.get_symbolizer(&Source::Kernel));
    let (exporter, rx) = McpExporter::new(
        10,
        JsonFormatter {
            symbolizer,
            source: Source::Kernel,
        },
    );

    let mut collector = SkbdropCollector::new(
        &mut open_project,
        probe_registry.as_ref(),
        params.to_config()?,
        exporter,
    )
    .map_err(MCPError::from)?;
    collector.attach_probe().map_err(MCPError::from)?;

    //  等待10分钟，如果10分钟内抓不到包就退出
    receive_event_blocking(collector, rx, timeout, cancel)
}

fn receive_event_blocking(
    mut collector: impl Collector,
    mut rx: Receiver<String>,
    timeout: Duration,
    cancel: CancellationToken,
) -> Result<Option<String>, MCPError> {
    let deadline = std::time::Instant::now() + timeout;

    while std::time::Instant::now() < deadline && !cancel.is_cancelled() {
        if let Ok(event) = rx.try_recv() {
            return Ok(Some(event));
        }

        collector.poll(Duration::from_millis(100))?;
    }

    Ok(None)
}

pub struct McpExporter<T, F> {
    event_tx: Sender<String>,
    formatter: F,
    _marked: PhantomData<T>,
}

impl<T: Sized + Send + Clone, F: StreamFormatter<T>> McpExporter<T, F> {
    pub(crate) fn new(capacity: usize, formatter: F) -> (Self, Receiver<String>) {
        let (event_tx, event_rs) = channel::<String>(capacity);
        (
            Self {
                formatter,
                event_tx,
                _marked: PhantomData,
            },
            event_rs,
        )
    }
}

impl<T: Sized + Send + Clone, F: StreamFormatter<T>> Exporter<T> for McpExporter<T, F> {
    fn dispatch(&mut self, event: T) {
        let mut buffer = Vec::new();
        if self.formatter.format(&mut buffer, &event).is_err() {
            return;
        }
        let Ok(event) = String::from_utf8(buffer) else {
            return;
        };
        let _ = self.event_tx.try_send(event);
    }
}

pub struct JsonFormatter<'a, S> {
    symbolizer: S,
    source: Source<'a>,
}

impl<S: Symbolizer> StreamFormatter<SkbdropEvent> for JsonFormatter<'_, S> {
    fn format<W: Write>(&self, w: &mut W, event: &SkbdropEvent) -> io::Result<()> {
        serde_json::to_writer(
            w,
            &SymbolizedSkbdropEvent {
                event,
                symbolizer: &self.symbolizer,
                source: self.source.clone(),
            },
        )
        .map_err(io::Error::other)
    }
}

struct SymbolizedSkbdropEvent<'a, S> {
    event: &'a SkbdropEvent,
    symbolizer: &'a S,
    source: Source<'a>,
}

#[derive(Serialize)]
struct SymbolizedStackFrame {
    addr: u64,
    name: String,
    start_addr: u64,
    offset: usize,
}

impl<S: Symbolizer> Serialize for SymbolizedSkbdropEvent<'_, S> {
    fn serialize<Ser>(&self, serializer: Ser) -> Result<Ser::Ok, Ser::Error>
    where
        Ser: serde::Serializer,
    {
        // Reuse the event's own serde output so l2/l3/l4 and future event fields stay intact.
        let mut value = serde_json::to_value(self.event).map_err(Ser::Error::custom)?;

        let stack_len = stack_len(self.event);
        if stack_len > 0 {
            let frames = self.event.stack[..stack_len]
                .iter()
                .copied()
                .map(|addr| {
                    let symbol = self.symbolizer.resolve(SymbolizeInput {
                        source: self.source.clone(),
                        addr,
                    });

                    SymbolizedStackFrame {
                        addr,
                        name: symbol.name.into_owned(),
                        start_addr: symbol.start_addr,
                        offset: symbol.offset,
                    }
                })
                .collect::<Vec<_>>();

            let value = value.as_object_mut().ok_or_else(|| {
                Ser::Error::custom("skbdrop event serializer must produce a JSON object")
            })?;
            value.insert(
                "stack".to_string(),
                serde_json::to_value(frames).map_err(Ser::Error::custom)?,
            );
        }

        value.serialize(serializer)
    }
}

fn stack_len(event: &SkbdropEvent) -> usize {
    if event.stack_size <= 0 {
        return 0;
    }

    ((event.stack_size as usize) / mem::size_of::<u64>()).min(event.stack.len())
}
