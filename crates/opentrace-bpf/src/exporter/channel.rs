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
        (
            Self {
                _event_tx: event_tx,
            },
            event_rx,
        )
    }
}

impl<T, E: Into<T>> Exporter<E> for SimpleUnboundChannelExporter<T, E> {
    fn dispatch(&mut self, event: E) {
        let _ = self.event_tx.send(event.into());
    }
}

#[cfg(test)]
mod tests {
    use tokio_test::block_on;

    use super::{SimpleBoundChannelExpoter, SimpleUnboundChannelExporter};
    use crate::exporter::Exporter;

    #[test]
    fn unbounded_exporter_sends_converted_events() {
        let (mut exporter, mut rx) = SimpleUnboundChannelExporter::<String, &str>::new();

        exporter.dispatch("hello");

        let received = block_on(rx.recv()).unwrap();
        assert_eq!(received, "hello");
    }

    #[test]
    fn bounded_exporter_creates_receiver_with_requested_capacity() {
        let (_exporter, rx) = SimpleBoundChannelExpoter::<u32>::new(8);

        assert_eq!(rx.max_capacity(), 8);
    }

    #[test]
    fn unbounded_channel_collects_multiple_events() {
        let (mut exporter, mut rx) = SimpleUnboundChannelExporter::<String, &str>::new();

        exporter.dispatch("a");
        exporter.dispatch("b");
        exporter.dispatch("c");

        assert_eq!(block_on(rx.recv()).unwrap(), "a");
        assert_eq!(block_on(rx.recv()).unwrap(), "b");
        assert_eq!(block_on(rx.recv()).unwrap(), "c");
    }

    #[test]
    fn unbounded_channel_converts_event_with_into() {
        let (mut exporter, mut rx) = SimpleUnboundChannelExporter::<u64, u32>::new();

        exporter.dispatch(42u32);

        assert_eq!(block_on(rx.recv()).unwrap(), 42u64);
    }

    #[test]
    fn bounded_channel_default_capacity() {
        let (_exporter, rx) = SimpleBoundChannelExpoter::<u32>::new(1);

        assert_eq!(rx.max_capacity(), 1);
    }
}
