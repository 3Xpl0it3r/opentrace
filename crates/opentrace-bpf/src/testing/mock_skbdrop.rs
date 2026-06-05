// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.

//! Mock SkbdropCollector 实现，用于测试网络丢包监控

use std::collections::VecDeque;
use std::time::Duration;

use crate::EbpfError;
use crate::collector::Collector;
use crate::collectors::net::SkbdropEvent;
use crate::types::net::{Addr, L2Info, L3Info, L4Info};

/// Mock SkbdropCollector，用于测试
///
/// # 示例
///
/// ```rust
/// use opentrace_bpf::testing::MockSkbdropCollector;
/// use opentrace_bpf::collectors::Collector;
/// use opentrace_bpf::ProbeRegistry;
/// use std::time::Duration;
///
/// let (mut collector, _rx) = MockSkbdropCollector::new();
/// let registry = ProbeRegistry::from_test_data();
/// collector.attach_probe(&registry).unwrap();
/// collector.poll(Duration::from_millis(100)).unwrap();
/// ```
pub struct MockSkbdropCollector {
    events: VecDeque<SkbdropEvent>,
    poll_count: usize,
    attach_count: usize,
    poll_error: Option<EbpfError>,
    attach_error: Option<EbpfError>,
}

impl MockSkbdropCollector {
    /// 创建新的 MockSkbdropCollector
    pub fn new() -> (Self, std::sync::mpsc::Receiver<SkbdropEvent>) {
        let (_tx, rx) = std::sync::mpsc::channel();
        (
            Self {
                events: VecDeque::new(),
                poll_count: 0,
                attach_count: 0,
                poll_error: None,
                attach_error: None,
            },
            rx,
        )
    }

    /// 添加事件到队列
    pub fn push_event(&mut self, event: SkbdropEvent) {
        self.events.push_back(event);
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
}

impl Default for MockSkbdropCollector {
    fn default() -> Self {
        Self::new().0
    }
}

impl Collector for MockSkbdropCollector {
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

/// 创建测试用的 SkbdropEvent
pub fn make_skbdrop_event(
    src_ip: [u8; 4],
    dst_ip: [u8; 4],
    sport: u16,
    dport: u16,
    drop_source: u8,
) -> SkbdropEvent {
    use crate::collectors::net::SkbdropEvent;

    SkbdropEvent {
        l2_info: L2Info { eth_proto: 0x0800 },
        l3_info: L3Info {
            saddr: Addr {
                v4addr: u32::from_ne_bytes(src_ip),
            },
            daddr: Addr {
                v4addr: u32::from_ne_bytes(dst_ip),
            },
            tot_len: 0,
            ip_version: 4,
            l4_proto: 6,
        },
        l4_info: L4Info {
            sport: u16::to_be(sport),
            dport: u16::to_be(dport),
            tcpflags: 0,
        },
        stack_size: 0,
        stack: [0; 16],
        drop_reason: 0,
        drop_source,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mock_skbdrop_collector_default_returns_ok() {
        let (mut collector, _rx) = MockSkbdropCollector::new();
        let registry = crate::ProbeRegistry::from_test_data();
        assert!(collector.attach_probe(&registry).is_ok());
        assert!(collector.poll(Duration::from_millis(100)).is_ok());
    }

    #[test]
    fn mock_skbdrop_collector_counts_calls() {
        let (mut collector, _rx) = MockSkbdropCollector::new();
        let registry = crate::ProbeRegistry::from_test_data();
        collector.attach_probe(&registry).unwrap();
        collector.poll(Duration::from_millis(100)).unwrap();
        collector.poll(Duration::from_millis(100)).unwrap();

        assert_eq!(collector.attach_count(), 1);
        assert_eq!(collector.poll_count(), 2);
    }

    #[test]
    fn mock_skbdrop_collector_returns_predefined_error() {
        let (collector, _rx) = MockSkbdropCollector::new();
        let mut collector = collector.with_poll_error(EbpfError::Other("poll failed".into()));

        assert!(collector.poll(Duration::from_millis(100)).is_err());
    }
}
