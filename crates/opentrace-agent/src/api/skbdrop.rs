// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.

use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use std::time::Duration;

use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::{IntoResponse, Response};
use tokio::sync::mpsc;
use tokio_stream::Stream;
use tokio_stream::wrappers::ReceiverStream;

use super::ApiResource;
use crate::errors::AgntError;
use crate::exporter::{SkbdropExporter, SkbdropRequest};
use crate::manager::Manager;
use crate::sink::SseRecord;

const WATCH_SINK_QUEUE_CAPACITY: usize = 1024;
const WATCH_KEEP_ALIVE_INTERVAL: Duration = Duration::from_secs(15);

pub struct SkbdropResource;

impl ApiResource for SkbdropResource {
    type Request = SkbdropRequest;

    fn resource_name() -> &'static str {
        "skbdrop"
    }

    async fn start(manager: Arc<Manager>, req: Self::Request) -> Result<Response, AgntError> {
        if req.watch.unwrap_or(false) {
            return Self::watch(manager, req).await;
        }

        if let Some(sink_name) = req.sink_name.clone() {
            let sink_tx = manager.get_sink(&sink_name).await?;
            let exporter = SkbdropExporter::with_sink(req, sink_tx, sink_name)?;
            manager.start("skbdrop", exporter).await?;
        } else {
            let exporter = SkbdropExporter::with_prometheus_metrics(req)?;
            manager.start("skbdrop", exporter).await?;
        }
        Ok(axum::http::StatusCode::CREATED.into_response())
    }

    async fn stop(manager: Arc<Manager>) -> Result<(), AgntError> {
        manager.stop("skbdrop").await?;
        Ok(())
    }
}

impl SkbdropResource {
    async fn watch(manager: Arc<Manager>, req: SkbdropRequest) -> Result<Response, AgntError> {
        let collector_name = Self::resource_name();
        let (tx, rx) = mpsc::channel::<SseRecord>(WATCH_SINK_QUEUE_CAPACITY);
        let exporter = SkbdropExporter::with_sse_sink(req, tx);
        manager
            .start(collector_name, exporter)
            .await
            .map_err(|err| match err {
                AgntError::AlreadyExists(_) => AgntError::AlreadyExists(format!(
                    "trace '{collector_name}' 已经启动，需要先暂停"
                )),
                err => err,
            })?;

        let stream = WatchSseStream::new(rx, manager, collector_name);
        let response = Sse::new(stream)
            .keep_alive(KeepAlive::new().interval(WATCH_KEEP_ALIVE_INTERVAL))
            .into_response();

        Ok(response)
    }
}

struct WatchSseStream {
    rx: ReceiverStream<SseRecord>,
    manager: Arc<Manager>,
    collector_name: &'static str,
    stop_on_drop: bool,
}

impl Unpin for WatchSseStream {}

impl WatchSseStream {
    fn new(
        rx: mpsc::Receiver<SseRecord>,
        manager: Arc<Manager>,
        collector_name: &'static str,
    ) -> Self {
        Self {
            rx: ReceiverStream::new(rx),
            manager,
            collector_name,
            stop_on_drop: true,
        }
    }

    fn stop_collector(&mut self) {
        if !self.stop_on_drop {
            return;
        }
        self.stop_on_drop = false;

        let manager = Arc::clone(&self.manager);
        let collector_name = self.collector_name;
        if let Ok(runtime) = tokio::runtime::Handle::try_current() {
            let cleanup = runtime.spawn(async move {
                _ = manager.stop(collector_name).await;
            });
            drop(cleanup);
        }
    }
}

impl Stream for WatchSseStream {
    type Item = Result<Event, AgntError>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();
        match Pin::new(&mut this.rx).poll_next(cx) {
            Poll::Ready(Some(record)) => Poll::Ready(Some(Ok(Event::default()
                .event(record.event())
                .data(record.data().to_owned())))),
            Poll::Ready(None) => {
                this.stop_collector();
                Poll::Ready(None)
            }
            Poll::Pending => Poll::Pending,
        }
    }
}

impl Drop for WatchSseStream {
    fn drop(&mut self) {
        self.stop_collector();
    }
}
