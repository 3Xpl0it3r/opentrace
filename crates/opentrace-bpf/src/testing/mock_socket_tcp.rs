// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.

//! Mock SocketTcpCollector 实现，用于测试 TCP socket 监控

use std::collections::VecDeque;
use std::time::Duration;

use crate::EbpfError;
use crate::collector::Collector;
use crate::collectors::net::SocketTcpEvent;
use crate::types::net::Addr;

/// Mock SocketTcpCollector，用于测试
///
/// # 示例
///
/// ```rust
/// use opentrace_bpf::testing::MockSocketTcpCollector;
/// use opentrace_bpf::collectors::Collector;
/// use opentrace_bpf::ProbeRegistry;
/// use std::time::Duration;
///
/// let mut collector = MockSocketTcpCollector::new();
/// let registry = ProbeRegistry::from_test_data();
/// collector.attach_probe(&registry).unwrap();
/// collector.poll(Duration::from_millis(100)).unwrap();
/// ```
pub struct MockSocketTcpCollector {
    events: VecDeque<SocketTcpEvent>,
    poll_count: usize,
    attach_count: usize,
    poll_error: Option<EbpfError>,
    attach_error: Option<EbpfError>,
}

impl MockSocketTcpCollector {
    /// 创建新的 MockSocketTcpCollector
    pub fn new() -> Self {
        Self {
            events: VecDeque::new(),
            poll_count: 0,
            attach_count: 0,
            poll_error: None,
            attach_error: None,
        }
    }

    /// 添加事件到队列
    pub fn push_event(&mut self, event: SocketTcpEvent) {
        self.events.push_back(event);
    }

    /// 弹出下一个事件
    pub fn pop_event(&mut self) -> Option<SocketTcpEvent> {
        self.events.pop_front()
    }

    /// 设置 poll 错误
    pub fn with_poll_error(mut self, error: EbpfError) -> Self {
        self.poll_error = Some(error);
        self
    }

    /// 设置 attach 错误
    pub fn with_attach_error(mut self, error: EbpfError) -> Self {
        self.attach_error = Some(error);
        self
    }

    /// 获取 poll 调用次数
    pub fn poll_count(&self) -> usize {
        self.poll_count
    }

    /// 获取 attach_probe 调用次数
    pub fn attach_count(&self) -> usize {
        self.attach_count
    }

    /// 获取剩余事件数量
    pub fn pending_events(&self) -> usize {
        self.events.len()
    }
}

impl Default for MockSocketTcpCollector {
    fn default() -> Self {
        Self::new()
    }
}

impl Collector for MockSocketTcpCollector {
    fn poll(&mut self, _interval: Duration) -> Result<(), EbpfError> {
        self.poll_count += 1;

        if let Some(err) = self.poll_error.take() {
            return Err(err);
        }

        Ok(())
    }

    fn attach_probe(&mut self, _probe_registry: &crate::ProbeRegistry) -> Result<(), EbpfError> {
        self.attach_count += 1;

        if let Some(err) = self.attach_error.take() {
            return Err(err);
        }

        Ok(())
    }
}

/// 创建测试用的 SocketTcpEvent
pub fn make_socket_tcp_event(
    remote_ip: [u8; 4],
    remote_port: u16,
    req_body: Option<&str>,
    resp_body: Option<&str>,
    timestamp: u64,
    duration: u64,
) -> SocketTcpEvent {
    SocketTcpEvent {
        remote_addr: Addr {
            v4addr: u32::from_ne_bytes(remote_ip),
        },
        remote_port,
        family: 2, // AF_INET
        req_body: req_body.map(|s| s.into()),
        resp_body: resp_body.map(|s| s.into()),
        timestamp,
        duration,
        request_size: req_body.map(|s| s.len() as u32).unwrap_or(0),
        response_size: resp_body.map(|s| s.len() as u32).unwrap_or(0),
        target: None,
    }
}

/// 创建带 target 的 SocketTcpEvent
pub fn make_socket_tcp_event_with_target(
    remote_ip: [u8; 4],
    remote_port: u16,
    req_body: Option<&str>,
    resp_body: Option<&str>,
    target: &str,
    timestamp: u64,
    duration: u64,
) -> SocketTcpEvent {
    SocketTcpEvent {
        remote_addr: Addr {
            v4addr: u32::from_ne_bytes(remote_ip),
        },
        remote_port,
        family: 2,
        req_body: req_body.map(|s| s.into()),
        resp_body: resp_body.map(|s| s.into()),
        timestamp,
        duration,
        request_size: req_body.map(|s| s.len() as u32).unwrap_or(0),
        response_size: resp_body.map(|s| s.len() as u32).unwrap_or(0),
        target: Some(target.into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mock_socket_tcp_collector_default_returns_ok() {
        let mut collector = MockSocketTcpCollector::new();
        let registry = crate::ProbeRegistry::from_test_data();
        assert!(collector.attach_probe(&registry).is_ok());
        assert!(collector.poll(Duration::from_millis(100)).is_ok());
    }

    #[test]
    fn mock_socket_tcp_collector_counts_calls() {
        let mut collector = MockSocketTcpCollector::new();
        let registry = crate::ProbeRegistry::from_test_data();
        collector.attach_probe(&registry).unwrap();
        collector.poll(Duration::from_millis(100)).unwrap();
        collector.poll(Duration::from_millis(100)).unwrap();

        assert_eq!(collector.attach_count(), 1);
        assert_eq!(collector.poll_count(), 2);
    }

    #[test]
    fn mock_socket_tcp_collector_push_pop_events() {
        let mut collector = MockSocketTcpCollector::new();
        let event = make_socket_tcp_event([10, 0, 0, 1], 8080, Some("request"), None, 100, 0);
        collector.push_event(event);

        assert_eq!(collector.pending_events(), 1);
        let popped = collector.pop_event().unwrap();
        assert_eq!(popped.remote_port, 8080);
        assert_eq!(collector.pending_events(), 0);
    }
}
