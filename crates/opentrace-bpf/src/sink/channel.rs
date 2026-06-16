// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.

use super::EventSink;

pub struct UnboundedChannelSink<E> {
    event_tx: tokio::sync::mpsc::UnboundedSender<E>,
}

impl<E> UnboundedChannelSink<E> {
    pub fn new() -> (Self, tokio::sync::mpsc::UnboundedReceiver<E>) {
        let (event_tx, event_rx) = tokio::sync::mpsc::unbounded_channel::<E>();
        (Self { event_tx }, event_rx)
    }
}

impl<E> EventSink<E> for UnboundedChannelSink<E> {
    fn dispatch(&mut self, event: E) {
        let _ = self.event_tx.send(event);
    }
}

pub struct BoundedChannelSink<E> {
    event_tx: tokio::sync::mpsc::Sender<E>,
}

impl<E> BoundedChannelSink<E> {
    pub fn new(capacity: usize) -> (Self, tokio::sync::mpsc::Receiver<E>) {
        let (event_tx, event_rx) = tokio::sync::mpsc::channel::<E>(capacity);
        (Self { event_tx }, event_rx)
    }
}

/* impl<T, E: Into<T>> EventSink<E> for UnboundedChannelSink<T, E> {
    fn dispatch(&mut self, event: E) {
        let _ = self.event_tx.send(event.into());
    }
} */

impl<E> EventSink<E> for BoundedChannelSink<E> {
    fn dispatch(&mut self, event: E) {
        let _ = self.event_tx.try_send(event);
    }
}

#[cfg(test)]
mod tests {
    use tokio_test::block_on;

    use super::{BoundedChannelSink, UnboundedChannelSink};
    use crate::sink::EventSink;

    #[test]
    fn unbounded_sink_sends_converted_events() {
        let (mut sink, mut rx) = UnboundedChannelSink::<String, &str>::new();

        sink.dispatch("hello");

        let received = block_on(rx.recv()).unwrap();
        assert_eq!(received, "hello");
    }

    #[test]
    fn bounded_sink_creates_receiver_with_requested_capacity() {
        let (_sink, rx) = BoundedChannelSink::<u32>::new(8);

        assert_eq!(rx.max_capacity(), 8);
    }

    #[test]
    fn bounded_channel_sends_events() {
        let (mut sink, mut rx) = BoundedChannelSink::<u32>::new(8);

        sink.dispatch(42);

        assert_eq!(block_on(rx.recv()).unwrap(), 42);
    }

    #[test]
    fn unbounded_channel_collects_multiple_events() {
        let (mut sink, mut rx) = UnboundedChannelSink::<String, &str>::new();

        sink.dispatch("a");
        sink.dispatch("b");
        sink.dispatch("c");

        assert_eq!(block_on(rx.recv()).unwrap(), "a");
        assert_eq!(block_on(rx.recv()).unwrap(), "b");
        assert_eq!(block_on(rx.recv()).unwrap(), "c");
    }

    #[test]
    fn unbounded_channel_converts_event_with_into() {
        let (mut sink, mut rx) = UnboundedChannelSink::<u64, u32>::new();

        sink.dispatch(42u32);

        assert_eq!(block_on(rx.recv()).unwrap(), 42u64);
    }

    #[test]
    fn bounded_channel_default_capacity() {
        let (_sink, rx) = BoundedChannelSink::<u32>::new(1);

        assert_eq!(rx.max_capacity(), 1);
    }
}
