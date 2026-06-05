// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.

//! Mock ProfileCollector 实现，用于测试 CPU 性能分析

use std::collections::VecDeque;
use std::time::Duration;

use crate::EbpfError;
use crate::collector::Collector;
use crate::collectors::cpu::ProfileEvent;

/// Mock ProfileCollector，用于测试
///
/// # 示例
///
/// ```rust
/// use opentrace_bpf::testing::MockProfileCollector;
/// use opentrace_bpf::collectors::Collector;
/// use opentrace_bpf::ProbeRegistry;
/// use std::time::Duration;
///
/// let mut collector = MockProfileCollector::new();
/// let registry = ProbeRegistry::from_test_data();
/// collector.attach_probe(&registry).unwrap();
/// collector.poll(Duration::from_millis(100)).unwrap();
/// ```
pub struct MockProfileCollector {
    events: VecDeque<ProfileEvent>,
    poll_count: usize,
    attach_count: usize,
    poll_error: Option<EbpfError>,
    attach_error: Option<EbpfError>,
}

impl MockProfileCollector {
    /// 创建新的 MockProfileCollector
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
    pub fn push_event(&mut self, event: ProfileEvent) {
        self.events.push_back(event);
    }

    /// 弹出下一个事件
    pub fn pop_event(&mut self) -> Option<ProfileEvent> {
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

impl Default for MockProfileCollector {
    fn default() -> Self {
        Self::new()
    }
}

impl Collector for MockProfileCollector {
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

/// 创建测试用的 ProfileEvent
pub fn make_profile_event(ustack: Vec<u64>, kstack: Vec<u64>) -> ProfileEvent {
    ProfileEvent { ustack, kstack }
}

/// 创建单栈 ProfileEvent
pub fn make_profile_event_single(addr: u64) -> ProfileEvent {
    ProfileEvent {
        ustack: vec![addr],
        kstack: vec![],
    }
}

/// 创建双栈 ProfileEvent
pub fn make_profile_event_both(user_addr: u64, kernel_addr: u64) -> ProfileEvent {
    ProfileEvent {
        ustack: vec![user_addr],
        kstack: vec![kernel_addr],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mock_profile_collector_default_returns_ok() {
        let mut collector = MockProfileCollector::new();
        let registry = crate::ProbeRegistry::from_test_data();
        assert!(collector.attach_probe(&registry).is_ok());
        assert!(collector.poll(Duration::from_millis(100)).is_ok());
    }

    #[test]
    fn mock_profile_collector_counts_calls() {
        let mut collector = MockProfileCollector::new();
        let registry = crate::ProbeRegistry::from_test_data();
        collector.attach_probe(&registry).unwrap();
        collector.attach_probe(&registry).unwrap();
        collector.poll(Duration::from_millis(100)).unwrap();

        assert_eq!(collector.attach_count(), 2);
        assert_eq!(collector.poll_count(), 1);
    }

    #[test]
    fn mock_profile_collector_push_pop_events() {
        let mut collector = MockProfileCollector::new();
        let event = make_profile_event_single(0x1234);
        collector.push_event(event);

        assert_eq!(collector.pending_events(), 1);
        let popped = collector.pop_event().unwrap();
        assert_eq!(popped.ustack, vec![0x1234]);
        assert_eq!(collector.pending_events(), 0);
    }
}
