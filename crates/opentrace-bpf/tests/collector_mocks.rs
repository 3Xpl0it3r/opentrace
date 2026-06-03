// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.

//! 集成测试：Collector mock 测试
//!
//! 测试 MockSkbdropCollector、MockSocketTcpCollector、MockProfileCollector

use opentrace_bpf::collector::Collector;
use opentrace_bpf::testing::{
    MockProfileCollector, MockSkbdropCollector, MockSocketTcpCollector, make_profile_event,
    make_profile_event_both, make_profile_event_single, make_skbdrop_event, make_socket_tcp_event,
    make_socket_tcp_event_with_target,
};
use opentrace_bpf::{EbpfError, collector};
use std::time::Duration;

// ==================== MockSkbdropCollector 测试 ====================

#[test]
fn mock_skbdrop_collector_basic_operations() {
    let (mut collector, _rx) = MockSkbdropCollector::new();

    assert!(collector.attach_probe().is_ok());
    assert!(collector.poll(Duration::from_millis(100)).is_ok());
    assert_eq!(collector.attach_count(), 1);
    assert_eq!(collector.poll_count(), 1);
}

#[test]
fn mock_skbdrop_collector_with_error() {
    let (collector, _rx) = MockSkbdropCollector::new();
    let mut collector = collector.with_poll_error(EbpfError::Other("test error".into()));

    assert!(collector.poll(Duration::from_millis(100)).is_err());
    // 第二次 poll 应该成功（错误只触发一次）
    assert!(collector.poll(Duration::from_millis(100)).is_ok());
}

// ==================== MockSocketTcpCollector 测试 ====================

#[test]
fn mock_socket_tcp_collector_basic_operations() {
    let mut collector = MockSocketTcpCollector::new();

    assert!(collector.attach_probe().is_ok());
    assert!(collector.poll(Duration::from_millis(100)).is_ok());
    assert_eq!(collector.attach_count(), 1);
    assert_eq!(collector.poll_count(), 1);
}

#[test]
fn mock_socket_tcp_collector_push_pop_events() {
    let mut collector = MockSocketTcpCollector::new();

    let event1 = make_socket_tcp_event([10, 0, 0, 1], 8080, Some("req1"), None, 100, 0);
    let event2 = make_socket_tcp_event([10, 0, 0, 2], 9090, Some("req2"), None, 200, 0);

    collector.push_event(event1);
    collector.push_event(event2);

    assert_eq!(collector.pending_events(), 2);

    let popped = collector.pop_event().unwrap();
    assert_eq!(popped.remote_port, 8080);

    let popped = collector.pop_event().unwrap();
    assert_eq!(popped.remote_port, 9090);

    assert!(collector.pop_event().is_none());
    assert_eq!(collector.pending_events(), 0);
}

// ==================== MockProfileCollector 测试 ====================

#[test]
fn mock_profile_collector_basic_operations() {
    let mut collector = MockProfileCollector::new();

    assert!(collector.attach_probe().is_ok());
    assert!(collector.poll(Duration::from_millis(100)).is_ok());
    assert_eq!(collector.attach_count(), 1);
    assert_eq!(collector.poll_count(), 1);
}

#[test]
fn mock_profile_collector_push_pop_events() {
    let mut collector = MockProfileCollector::new();

    let event1 = make_profile_event_single(0x1000);
    let event2 = make_profile_event_both(0x2000, 0x3000);

    collector.push_event(event1);
    collector.push_event(event2);

    assert_eq!(collector.pending_events(), 2);

    let popped = collector.pop_event().unwrap();
    assert_eq!(popped.ustack, vec![0x1000]);
    assert!(popped.kstack.is_empty());

    let popped = collector.pop_event().unwrap();
    assert_eq!(popped.ustack, vec![0x2000]);
    assert_eq!(popped.kstack, vec![0x3000]);

    assert!(collector.pop_event().is_none());
}

// ==================== 错误处理测试 ====================

#[test]
fn mock_collectors_error_recovery() {
    let (collector, _) = MockSkbdropCollector::new();
    let collector = collector.with_poll_error(EbpfError::Other("error".into()));
    let mut collector = collector;

    // 第一次 poll 失败
    assert!(collector.poll(Duration::from_millis(100)).is_err());
    // 第二次 poll 成功
    assert!(collector.poll(Duration::from_millis(100)).is_ok());
}

#[test]
fn mock_collectors_attach_error() {
    let (collector, _) = MockSkbdropCollector::new();
    let collector = collector.with_attach_error(EbpfError::Other("attach error".into()));
    let mut collector = collector;

    assert!(collector.attach_probe().is_err());
    assert!(collector.attach_probe().is_ok());
}
