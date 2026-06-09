// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.

use opentrace_bpf::sinks::EventSink;

use super::KafkaSink;

pub enum CollectorSink<T, P: EventSink<T>> {
    Prometheus(P),
    Kafka(KafkaSink<T>),
}

impl<T, P: EventSink<T>> EventSink<T> for CollectorSink<T, P> {
    fn dispatch(&mut self, event: T) {
        match self {
            CollectorSink::Prometheus(s) => s.dispatch(event),
            CollectorSink::Kafka(s) => s.dispatch(event),
        }
    }
}
