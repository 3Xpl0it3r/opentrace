// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.

use std::marker::PhantomData;
use std::str::from_utf8_unchecked;
use std::time::Duration;

use rmcp::model::{CallToolResult, Content};
use rmcp::serde_json;
use serde::Serialize;
use tokio::sync::mpsc::{Receiver, Sender, channel};

use opentrace_bpf::collector::Collector;
use opentrace_bpf::format::{Formatter, JsonFormatter};
use opentrace_bpf::{Exporter, symbol::SymbolResolver};

use crate::errors::MCPError;

pub struct McpExporter<T, F, R> {
    event_tx: Sender<String>,
    formatter: F,
    resolver: R,
    _marked: PhantomData<T>,
}

impl<T: Sized + Send + Serialize + Clone, F: Formatter<T>, R: SymbolResolver> McpExporter<T, F, R> {
    pub(crate) fn new(capacity: usize, formatter: F, resolver: R) -> (Self, Receiver<String>) {
        let (event_tx, event_rs) = channel::<String>(capacity);
        (
            Self {
                resolver,
                formatter,
                event_tx,
                _marked: PhantomData,
            },
            event_rs,
        )
    }
}

impl<T: Serialize + Sized + Send + Clone, F: Formatter<T>, R: SymbolResolver> Exporter<T>
    for McpExporter<T, F, R>
{
    fn handle(&mut self, event: T) {
        let mut buffer = Vec::new();
        if self
            .formatter
            .format(&mut buffer, &event, &self.resolver)
            .is_err()
        {
            return;
        }
        let _ = self
            .event_tx
            .try_send(unsafe { String::from_utf8_unchecked(buffer) });
    }
    fn load(&self, data: &[u8]) -> T {
        unsafe { std::ptr::read(data.as_ptr() as *const T) }
    }
}

pub(crate) fn receive_event_sync<F: Formatter<String>>(
    mut collector: impl Collector,
    mut rx: Receiver<String>,
    timeout: Duration,
    formatter: F,
) -> Result<CallToolResult, MCPError> {
    tokio::task::block_in_place(|| {
        let deadline = std::time::Instant::now() + timeout;
        loop {
            if std::time::Instant::now() > deadline {
                return Ok(CallToolResult::success(vec![]));
            }
            match rx.try_recv() {
                Ok(event) => {
                    return Ok(CallToolResult::success(vec![Content::text(event)]));
                }
                Err(_) => {}
            }
            let _ = collector.poll(Duration::from_millis(100));
        }
    })
}
