// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.

use std::time::{Duration, Instant};

use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

use opentrace_bpf::format::StructeredFormatter;
use opentrace_bpf::sinks::BoundedChannelSink;

use crate::errors::AgntError;

const CACHE_QUEUE_CAPACITY: usize = 4096;
const SINK_QUEUE_CAPACITY: usize = 1024;
const CACHE_STOP_TIMEOUT: Duration = Duration::from_secs(5);

pub(crate) struct SinkCacher<T, F>
where
    F: StructeredFormatter<T> + Send + 'static,
    F::Output: Send + 'static,
{
    event_rx: mpsc::Receiver<T>,
    sink_tx: mpsc::Sender<F::Output>,
    formatter: F,
}

impl<T, F> SinkCacher<T, F>
where
    T: Send + 'static,
    F: StructeredFormatter<T> + Send + 'static,
    F::Output: Send + 'static,
{
    pub(crate) fn new(
        sink_tx: mpsc::Sender<F::Output>,
        formatter: F,
    ) -> (Self, BoundedChannelSink<T>) {
        let (channel_sink, rx) = BoundedChannelSink::new(CACHE_QUEUE_CAPACITY);
        (
            Self {
                event_rx: rx,
                sink_tx,
                formatter,
            },
            channel_sink,
        )
    }

    pub(crate) async fn run(mut self, cancel: CancellationToken) -> Result<(), AgntError> {
        loop {
            tokio::select! {
                _ = cancel.cancelled() => {
                    break;
                }
                event = self.event_rx.recv() => {
                    let Some(event) = event else {
                        break;
                    };
                    let record = self.formatter.format(event).map_err(AgntError::other)?;
                    self.sink_tx
                        .send(record)
                        .await
                        .map_err(|_| AgntError::other("sink record receiver closed"))?;
                }
            }
        }

        Ok(())
    }
}

pub(crate) struct SinkCacheTask {
    cancel: CancellationToken,
    handler: JoinHandle<Result<(), AgntError>>,
}

impl SinkCacheTask {
    pub(crate) fn new<T, F>(cache: SinkCacher<T, F>, cancel: CancellationToken) -> Self
    where
        T: Send + 'static,
        F: StructeredFormatter<T> + Send + 'static,
        F::Output: Send + 'static,
    {
        let handler = tokio::spawn(cache.run(cancel.clone()));
        Self { cancel, handler }
    }

    pub(crate) async fn stop(self) {
        self.stop_until(Instant::now() + CACHE_STOP_TIMEOUT).await;
    }

    async fn stop_until(mut self, deadline: Instant) {
        self.cancel.cancel();

        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            self.handler.abort();
            _ = self.handler.await;
            return;
        }

        let sleep = tokio::time::sleep(remaining);
        tokio::pin!(sleep);
        tokio::select! {
            _ = &mut self.handler => {}
            _ = &mut sleep => {
                self.handler.abort();
                _ = self.handler.await;
            }
        }
    }
}
