// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.

use std::thread;
use std::time::{Duration, Instant};

use tokio::sync::{mpsc, oneshot};
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

use opentrace_bpf::format::StructeredFormatter;
use opentrace_bpf::sinks::BoundedChannelSink;

use crate::errors::AgntError;

const CACHE_QUEUE_CAPACITY: usize = 4096;
const CACHE_STOP_TIMEOUT: Duration = Duration::from_secs(5);

pub(crate) struct SinkCacher<T, F>
where
    F: StructeredFormatter<T>,
{
    event_rx: mpsc::Receiver<T>,
    sink_tx: mpsc::Sender<F::Output>,
    formatter: F,
}

impl<T, F> SinkCacher<T, F>
where
    F: StructeredFormatter<T>,
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

    pub(crate) async fn run(self, cancel: CancellationToken) -> Result<(), AgntError> {
        run_cache_loop(self.event_rx, self.sink_tx, self.formatter, cancel).await
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

/// LocalSet-backed cache for formatters that own !Send symbolizers.
///
/// Keep this path limited to skbdrop/profile style stack symbolization. Ordinary
/// event formatters should use `SinkCacheTask`.
pub(crate) struct LocalSinkCacheTask {
    cancel: CancellationToken,
    done: oneshot::Receiver<Result<(), AgntError>>,
}

impl LocalSinkCacheTask {
    pub(crate) fn new<T, F, MakeFormatter>(
        sink_tx: mpsc::Sender<F::Output>,
        make_formatter: MakeFormatter,
        cancel: CancellationToken,
    ) -> (Self, BoundedChannelSink<T>)
    where
        T: Send + 'static,
        F: StructeredFormatter<T> + 'static,
        F::Output: Send + 'static,
        MakeFormatter: FnOnce() -> F + Send + 'static,
    {
        let (channel_sink, event_rx) = BoundedChannelSink::new(CACHE_QUEUE_CAPACITY);
        let (done_tx, done) = oneshot::channel();
        let thread_cancel = cancel.clone();

        let handle = thread::spawn(move || {
            let result = run_local_cache(event_rx, sink_tx, make_formatter, thread_cancel);
            _ = done_tx.send(result);
        });
        drop(handle);

        (Self { cancel, done }, channel_sink)
    }

    pub(crate) async fn stop(self) {
        self.stop_until(Instant::now() + CACHE_STOP_TIMEOUT).await;
    }

    async fn stop_until(mut self, deadline: Instant) {
        self.cancel.cancel();

        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return;
        }

        let sleep = tokio::time::sleep(remaining);
        tokio::pin!(sleep);
        tokio::select! {
            _ = &mut self.done => {}
            _ = &mut sleep => {}
        }
    }
}

fn run_local_cache<T, F, MakeFormatter>(
    event_rx: mpsc::Receiver<T>,
    sink_tx: mpsc::Sender<F::Output>,
    make_formatter: MakeFormatter,
    cancel: CancellationToken,
) -> Result<(), AgntError>
where
    F: StructeredFormatter<T> + 'static,
    F::Output: Send + 'static,
    MakeFormatter: FnOnce() -> F,
{
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(AgntError::other)?;
    let local = tokio::task::LocalSet::new();

    runtime.block_on(local.run_until(async move {
        let formatter = make_formatter();
        run_cache_loop(event_rx, sink_tx, formatter, cancel).await
    }))
}

async fn run_cache_loop<T, F>(
    mut event_rx: mpsc::Receiver<T>,
    sink_tx: mpsc::Sender<F::Output>,
    formatter: F,
    cancel: CancellationToken,
) -> Result<(), AgntError>
where
    F: StructeredFormatter<T>,
{
    loop {
        tokio::select! {
            _ = cancel.cancelled() => {
                break;
            }
            event = event_rx.recv() => {
                let Some(event) = event else {
                    break;
                };
                let record = formatter.format(event).map_err(AgntError::other)?;
                tokio::select! {
                    _ = cancel.cancelled() => {
                        break;
                    }
                    sent = sink_tx.send(record) => {
                        sent.map_err(|_| AgntError::other("sink record receiver closed"))?;
                    }
                }
            }
        }
    }

    Ok(())
}
