// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.

//! 集成测试：exporters 模块
//!
//! 测试导出器

use opentrace_bpf::exporters::{
    Exporter, SimpleBoundChannelExpoter, SimpleUnboundChannelExporter, StreamWriterExpoter,
};
use opentrace_bpf::format::StreamFormatter;
use std::io;

// ==================== StreamWriterExpoter 测试 ====================

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
fn stream_writer_exporter_dispatch() {
    let mut exporter = StreamWriterExpoter::new(Vec::new(), TestFormatter);
    exporter.dispatch(42);
    assert_eq!(exporter.writer, b"value=42");
}

#[test]
fn stream_writer_exporter_ignores_formatter_errors() {
    let mut exporter = StreamWriterExpoter::new(Vec::new(), FailingFormatter);
    exporter.dispatch(42);
    assert!(exporter.writer.is_empty());
}

#[test]
fn stream_writer_exporter_multiple_dispatches() {
    let mut exporter = StreamWriterExpoter::new(Vec::new(), TestFormatter);
    exporter.dispatch(1);
    exporter.dispatch(2);
    exporter.dispatch(3);
    assert_eq!(exporter.writer, b"value=1value=2value=3");
}

// ==================== SimpleUnboundChannelExporter 测试 ====================

#[test]
fn unbounded_exporter_dispatch() {
    let (mut exporter, mut rx) = SimpleUnboundChannelExporter::<String, &str>::new();
    exporter.dispatch("hello");
    // 使用 tokio-test 的 block_on
    let received = tokio_test::block_on(rx.recv()).unwrap();
    assert_eq!(received, "hello");
}

#[test]
fn unbounded_exporter_multiple_dispatches() {
    let (mut exporter, rx) = SimpleUnboundChannelExporter::<String, &str>::new();
    exporter.dispatch("a");
    exporter.dispatch("b");
    exporter.dispatch("c");
    drop(exporter); // 关闭发送端，让 recv 返回 None

    let mut received = Vec::new();
    let mut rx = rx;
    while let Some(item) = tokio_test::block_on(rx.recv()) {
        received.push(item);
    }
    assert_eq!(received, vec!["a", "b", "c"]);
}

// ==================== SimpleBoundChannelExpoter 测试 ====================

#[test]
fn bounded_exporter_capacity() {
    let (_exporter, rx) = SimpleBoundChannelExpoter::<u32>::new(8);
    assert_eq!(rx.max_capacity(), 8);
}

#[test]
fn bounded_exporter_default_capacity() {
    let (_exporter, rx) = SimpleBoundChannelExpoter::<u32>::new(1);
    assert_eq!(rx.max_capacity(), 1);
}
