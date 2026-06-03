// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.

//! 集成测试：sinks 模块
//!
//! 测试事件接收端

use opentrace_bpf::format::StreamFormatter;
use opentrace_bpf::sinks::{BoundedChannelSink, EventSink, StreamWriterSink, UnboundedChannelSink};
use std::io;

// ==================== StreamWriterSink 测试 ====================

#[derive(Default)]
struct TestFormatter;

impl StreamFormatter<u32> for TestFormatter {
    fn format<W: io::Write>(&self, w: &mut W, value: &u32) -> io::Result<()> {
        write!(w, "value={value}")
    }
}

#[derive(Default)]
struct FailingFormatter;

impl StreamFormatter<u32> for FailingFormatter {
    fn format<W: io::Write>(&self, _w: &mut W, _value: &u32) -> io::Result<()> {
        Err(io::Error::other("format failed"))
    }
}

#[test]
fn stream_writer_sink_dispatch() {
    let mut sink = StreamWriterSink::new(Vec::new(), TestFormatter);
    sink.dispatch(42);
    assert_eq!(sink.writer, b"value=42");
}

#[test]
fn stream_writer_sink_ignores_formatter_errors() {
    let mut sink = StreamWriterSink::new(Vec::new(), FailingFormatter);
    sink.dispatch(42);
    assert!(sink.writer.is_empty());
}

#[test]
fn stream_writer_sink_multiple_dispatches() {
    let mut sink = StreamWriterSink::new(Vec::new(), TestFormatter);
    sink.dispatch(1);
    sink.dispatch(2);
    sink.dispatch(3);
    assert_eq!(sink.writer, b"value=1value=2value=3");
}

// ==================== UnboundedChannelSink 测试 ====================

#[test]
fn unbounded_sink_dispatch() {
    let (mut sink, mut rx) = UnboundedChannelSink::<String, &str>::new();
    sink.dispatch("hello");
    // 使用 tokio-test 的 block_on
    let received = tokio_test::block_on(rx.recv()).unwrap();
    assert_eq!(received, "hello");
}

#[test]
fn unbounded_sink_multiple_dispatches() {
    let (mut sink, rx) = UnboundedChannelSink::<String, &str>::new();
    sink.dispatch("a");
    sink.dispatch("b");
    sink.dispatch("c");
    drop(sink); // 关闭发送端，让 recv 返回 None

    let mut received = Vec::new();
    let mut rx = rx;
    while let Some(item) = tokio_test::block_on(rx.recv()) {
        received.push(item);
    }
    assert_eq!(received, vec!["a", "b", "c"]);
}

// ==================== BoundedChannelSink 测试 ====================

#[test]
fn bounded_sink_capacity() {
    let (_sink, rx) = BoundedChannelSink::<u32>::new(8);
    assert_eq!(rx.max_capacity(), 8);
}

#[test]
fn bounded_sink_default_capacity() {
    let (_sink, rx) = BoundedChannelSink::<u32>::new(1);
    assert_eq!(rx.max_capacity(), 1);
}
