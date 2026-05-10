// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.

use std::marker::PhantomData;
use std::time::Duration;

use rmcp::model::{CallToolResult, Content};
use rmcp::serde_json;
use serde::Serialize;
use tokio::sync::mpsc::{Receiver, Sender, channel};

use opentrace_bpf::{EbpfProgram, Exporter};

use crate::errors::MCPError;

pub struct McpExporter<T: Sized + Send + Serialize + Clone> {
    event_tx: Sender<T>,
    _marker: PhantomData<T>,
}

impl<T: Sized + Send + Serialize + Clone> McpExporter<T> {
    pub(crate) fn with_capacity(capacity: usize) -> (Self, Receiver<T>) {
        let (tx, rx) = channel::<T>(capacity);
        (
            Self {
                event_tx: tx,
                _marker: PhantomData,
            },
            rx,
        )
    }
}

impl<T: Serialize + Sized + Send + Clone> Exporter<T> for McpExporter<T> {
    fn handle(&mut self, event: T) {
        let _ = self.event_tx.try_send(event);
    }
    fn load(&self, data: &[u8]) -> T {
        unsafe { std::ptr::read(data.as_ptr() as *const T) }
    }
}

pub(crate) fn receive_event_sync<T>(
    mut program: impl EbpfProgram,
    mut rx: Receiver<T>,
    timeout: Duration,
) -> Result<CallToolResult, MCPError>
where
    T: Clone + Serialize,
{
    tokio::task::block_in_place(|| {
        let deadline = std::time::Instant::now() + timeout;
        loop {
            if std::time::Instant::now() > deadline {
                return Ok(CallToolResult::success(vec![]));
            }
            match rx.try_recv() {
                Ok(event) => {
                    return Ok(CallToolResult::success(vec![Content::text(
                        serde_json::to_string(&event).unwrap_or_default(),
                    )]));
                }
                Err(_) => {}
            }
            let _ = program.poll(Duration::from_millis(100));
        }
    })
}
