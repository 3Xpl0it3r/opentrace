// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.

use std::marker::PhantomData;

use super::Exporter;

pub struct SimpleUnboundChannelExporter<T, E: Into<T>> {
    event_tx: tokio::sync::mpsc::UnboundedSender<T>,
    _phantom: PhantomData<E>,
}

impl<T, E: Into<T>> SimpleUnboundChannelExporter<T, E> {
    pub fn new() -> (Self, tokio::sync::mpsc::UnboundedReceiver<T>) {
        let (event_tx, event_rx) = tokio::sync::mpsc::unbounded_channel::<T>();
        (
            Self {
                event_tx,
                _phantom: PhantomData,
            },
            event_rx,
        )
    }
}

pub struct SimpleBoundChannelExpoter<E> {
    _event_tx: tokio::sync::mpsc::Sender<E>,
}

impl<E> SimpleBoundChannelExpoter<E> {
    pub fn new(capacity: usize) -> (Self, tokio::sync::mpsc::Receiver<E>) {
        let (event_tx, event_rx) = tokio::sync::mpsc::channel::<E>(capacity);
        (Self { _event_tx: event_tx }, event_rx)
    }
}

impl<T, E: Into<T>> Exporter<E> for SimpleUnboundChannelExporter<T, E> {
    fn dispatch(&mut self, event: E) {
        let _ = self.event_tx.send(event.into());
    }
}
