// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.

use crate::exporters::Exporter;

/// Mock Exporter 实现，用于测试
///
/// # 示例
///
/// ```rust
/// use opentrace_bpf::testing::MockExporter;
/// use opentrace_bpf::exporter::Exporter;
///
/// let mut exporter = MockExporter::<u32>::new();
/// exporter.dispatch(42);
/// exporter.dispatch(100);
///
/// assert_eq!(exporter.events(), &[42, 100]);
/// assert_eq!(exporter.dispatch_count(), 2);
/// ```
pub struct MockExporter<T> {
    events: Vec<T>,
    load_fn: Option<Box<dyn Fn(&[u8]) -> T>>,
}

impl<T> MockExporter<T> {
    /// 创建新的 MockExporter
    pub fn new() -> Self {
        Self {
            events: Vec::new(),
            load_fn: None,
        }
    }

    /// 设置自定义 load 函数
    pub fn with_load_fn(mut self, f: Box<dyn Fn(&[u8]) -> T>) -> Self {
        self.load_fn = Some(f);
        self
    }

    /// 获取所有 dispatch 的事件
    pub fn events(&self) -> &[T] {
        &self.events
    }

    /// 获取 dispatch 调用次数
    pub fn dispatch_count(&self) -> usize {
        self.events.len()
    }

    /// 清空事件列表
    pub fn clear(&mut self) {
        self.events.clear();
    }

    /// 检查是否包含满足条件的事件
    pub fn has_event<F>(&self, predicate: F) -> bool
    where
        F: Fn(&T) -> bool,
    {
        self.events.iter().any(predicate)
    }

    /// 查找满足条件的事件
    pub fn find_event<F>(&self, predicate: F) -> Option<&T>
    where
        F: Fn(&T) -> bool,
    {
        self.events.iter().find(|e| predicate(e))
    }
}

impl<T: Clone> MockExporter<T> {
    /// 获取最后一个事件
    pub fn last_event(&self) -> Option<&T> {
        self.events.last()
    }
}

impl<T: std::fmt::Debug> MockExporter<T> {
    /// 打印所有事件（调试用）
    pub fn dump_events(&self) {
        for (i, event) in self.events.iter().enumerate() {
            println!("[{}] {:?}", i, event);
        }
    }
}

impl<T> Default for MockExporter<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> Exporter<T> for MockExporter<T> {
    fn load(&self, data: &[u8]) -> T {
        match &self.load_fn {
            Some(f) => f(data),
            None => unsafe { std::ptr::read(data.as_ptr() as *const T) },
        }
    }

    fn dispatch(&mut self, event: T) {
        self.events.push(event);
    }
}

/// 用于测试的 NullExporter，丢弃所有事件
pub struct NullExporter;

impl<T> Exporter<T> for NullExporter {
    fn dispatch(&mut self, _event: T) {
        // 什么都不做
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mock_exporter_collects_events() {
        let mut exporter = MockExporter::<u32>::new();
        exporter.dispatch(1);
        exporter.dispatch(2);
        exporter.dispatch(3);

        assert_eq!(exporter.events(), &[1, 2, 3]);
        assert_eq!(exporter.dispatch_count(), 3);
    }

    #[test]
    fn mock_exporter_clear() {
        let mut exporter = MockExporter::<u32>::new();
        exporter.dispatch(1);
        exporter.dispatch(2);

        assert_eq!(exporter.dispatch_count(), 2);

        exporter.clear();
        assert_eq!(exporter.dispatch_count(), 0);
        assert!(exporter.events().is_empty());
    }

    #[test]
    fn mock_exporter_has_event() {
        let mut exporter = MockExporter::<u32>::new();
        exporter.dispatch(42);
        exporter.dispatch(100);

        assert!(exporter.has_event(|&e| e == 42));
        assert!(exporter.has_event(|&e| e > 50));
        assert!(!exporter.has_event(|&e| e == 0));
    }

    #[test]
    fn mock_exporter_find_event() {
        let mut exporter = MockExporter::<u32>::new();
        exporter.dispatch(10);
        exporter.dispatch(20);
        exporter.dispatch(30);

        let found = exporter.find_event(|&e| e > 15);
        assert_eq!(found, Some(&20));
    }

    #[test]
    fn mock_exporter_last_event() {
        let mut exporter = MockExporter::<u32>::new();
        assert!(exporter.last_event().is_none());

        exporter.dispatch(1);
        assert_eq!(exporter.last_event(), Some(&1));

        exporter.dispatch(2);
        assert_eq!(exporter.last_event(), Some(&2));
    }

    #[test]
    fn null_exporter_discards_events() {
        let mut exporter = NullExporter;
        exporter.dispatch(42);
        exporter.dispatch(100);
        // 没有 panic 就是成功
    }
}
