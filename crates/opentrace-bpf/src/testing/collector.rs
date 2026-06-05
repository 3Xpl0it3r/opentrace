// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.

use std::collections::VecDeque;
use std::time::Duration;

use crate::EbpfError;
use crate::collector::Collector;

/// Mock Collector 实现，用于测试
///
/// # 示例
///
/// ```rust
/// use opentrace_bpf::testing::MockCollector;
/// use opentrace_bpf::collectors::Collector;
/// use std::time::Duration;
///
/// let mut collector = MockCollector::new();
/// assert!(collector.poll(Duration::from_millis(100)).is_ok());
/// assert_eq!(collector.poll_count(), 1);
/// ```
pub struct MockCollector {
    poll_results: VecDeque<Result<(), EbpfError>>,
    attach_results: VecDeque<Result<(), EbpfError>>,
    poll_count: usize,
    attach_count: usize,
}

impl MockCollector {
    /// 创建新的 MockCollector，默认返回 Ok
    pub fn new() -> Self {
        Self {
            poll_results: VecDeque::new(),
            attach_results: VecDeque::new(),
            poll_count: 0,
            attach_count: 0,
        }
    }

    /// 添加 poll 调用的返回值
    pub fn with_poll_result(mut self, result: Result<(), EbpfError>) -> Self {
        self.poll_results.push_back(result);
        self
    }

    /// 添加多个 poll 调用的返回值
    pub fn with_poll_results(mut self, results: Vec<Result<(), EbpfError>>) -> Self {
        for r in results {
            self.poll_results.push_back(r);
        }
        self
    }

    /// 添加 attach_probe 调用的返回值
    pub fn with_attach_result(mut self, result: Result<(), EbpfError>) -> Self {
        self.attach_results.push_back(result);
        self
    }

    /// 添加多个 attach_probe 调用的返回值
    pub fn with_attach_results(mut self, results: Vec<Result<(), EbpfError>>) -> Self {
        for r in results {
            self.attach_results.push_back(r);
        }
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

    /// 重置调用计数
    pub fn reset_counts(&mut self) {
        self.poll_count = 0;
        self.attach_count = 0;
    }
}

impl Default for MockCollector {
    fn default() -> Self {
        Self::new()
    }
}

impl Collector for MockCollector {
    fn poll(&mut self, _interval: Duration) -> Result<(), EbpfError> {
        self.poll_count += 1;
        self.poll_results.pop_front().unwrap_or(Ok(()))
    }

    fn attach_probe(&mut self, _probe_registry: &crate::ProbeRegistry) -> Result<(), EbpfError> {
        self.attach_count += 1;
        self.attach_results.pop_front().unwrap_or(Ok(()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mock_collector_default_returns_ok() {
        let mut collector = MockCollector::new();
        let registry = crate::ProbeRegistry::from_test_data();
        assert!(collector.poll(Duration::from_millis(100)).is_ok());
        assert!(collector.attach_probe(&registry).is_ok());
    }

    #[test]
    fn mock_collector_counts_calls() {
        let mut collector = MockCollector::new();
        let registry = crate::ProbeRegistry::from_test_data();
        collector.poll(Duration::from_millis(100)).unwrap();
        collector.poll(Duration::from_millis(100)).unwrap();
        collector.attach_probe(&registry).unwrap();

        assert_eq!(collector.poll_count(), 2);
        assert_eq!(collector.attach_count(), 1);
    }

    #[test]
    fn mock_collector_returns_predefined_errors() {
        let mut collector = MockCollector::new()
            .with_poll_result(Err(EbpfError::Other("poll error".into())))
            .with_attach_result(Err(EbpfError::Other("attach error".into())));
        let registry = crate::ProbeRegistry::from_test_data();

        assert!(collector.poll(Duration::from_millis(100)).is_err());
        assert!(collector.attach_probe(&registry).is_err());
    }

    #[test]
    fn mock_collector_returns_sequential_results() {
        let mut collector = MockCollector::new().with_poll_results(vec![
            Ok(()),
            Err(EbpfError::Other("error".into())),
            Ok(()),
        ]);

        assert!(collector.poll(Duration::from_millis(100)).is_ok());
        assert!(collector.poll(Duration::from_millis(100)).is_err());
        assert!(collector.poll(Duration::from_millis(100)).is_ok());
    }

    #[test]
    fn mock_collector_reset_counts() {
        let mut collector = MockCollector::new();
        let registry = crate::ProbeRegistry::from_test_data();
        collector.poll(Duration::from_millis(100)).unwrap();
        collector.attach_probe(&registry).unwrap();

        assert_eq!(collector.poll_count(), 1);
        assert_eq!(collector.attach_count(), 1);

        collector.reset_counts();
        assert_eq!(collector.poll_count(), 0);
        assert_eq!(collector.attach_count(), 0);
    }
}
