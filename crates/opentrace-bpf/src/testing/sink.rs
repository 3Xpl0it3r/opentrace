// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.

use crate::sink::EventSink;

type LoadFn<T> = Box<dyn Fn(&[u8]) -> T>;

/// RecordingSink 实现，用于测试
///
/// # 示例
///
/// ```rust
/// use opentrace_bpf::testing::RecordingSink;
/// use opentrace_bpf::sinks::EventSink;
///
/// let mut sink = RecordingSink::<u32>::new();
/// sink.dispatch(42);
/// sink.dispatch(100);
///
/// assert_eq!(sink.events(), &[42, 100]);
/// assert_eq!(sink.dispatch_count(), 2);
/// ```
pub struct RecordingSink<T> {
    events: Vec<T>,
    load_fn: Option<LoadFn<T>>,
}

impl<T> RecordingSink<T> {
    /// 创建新的 RecordingSink
    pub fn new() -> Self {
        Self {
            events: Vec::new(),
            load_fn: None,
        }
    }

    /// 设置自定义 load 函数
    pub fn with_load_fn(mut self, f: LoadFn<T>) -> Self {
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

impl<T: Clone> RecordingSink<T> {
    /// 获取最后一个事件
    pub fn last_event(&self) -> Option<&T> {
        self.events.last()
    }
}

impl<T: std::fmt::Debug> RecordingSink<T> {
    /// 打印所有事件（调试用）
    pub fn dump_events(&self) {
        for (i, event) in self.events.iter().enumerate() {
            println!("[{}] {:?}", i, event);
        }
    }
}

impl<T> Default for RecordingSink<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> EventSink<T> for RecordingSink<T> {
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

#[cfg(test)]
mod tests {
    use super::*;

    pub struct NullSink;

    impl<T> EventSink<T> for NullSink {
        fn dispatch(&mut self, _event: T) {
            // 什么都不做
        }
    }

    #[test]
    fn recording_sink_collects_events() {
        let mut sink = RecordingSink::<u32>::new();
        sink.dispatch(1);
        sink.dispatch(2);
        sink.dispatch(3);

        assert_eq!(sink.events(), &[1, 2, 3]);
        assert_eq!(sink.dispatch_count(), 3);
    }

    #[test]
    fn recording_sink_clear() {
        let mut sink = RecordingSink::<u32>::new();
        sink.dispatch(1);
        sink.dispatch(2);

        assert_eq!(sink.dispatch_count(), 2);

        sink.clear();
        assert_eq!(sink.dispatch_count(), 0);
        assert!(sink.events().is_empty());
    }

    #[test]
    fn recording_sink_has_event() {
        let mut sink = RecordingSink::<u32>::new();
        sink.dispatch(42);
        sink.dispatch(100);

        assert!(sink.has_event(|&e| e == 42));
        assert!(sink.has_event(|&e| e > 50));
        assert!(!sink.has_event(|&e| e == 0));
    }

    #[test]
    fn recording_sink_find_event() {
        let mut sink = RecordingSink::<u32>::new();
        sink.dispatch(10);
        sink.dispatch(20);
        sink.dispatch(30);

        let found = sink.find_event(|&e| e > 15);
        assert_eq!(found, Some(&20));
    }

    #[test]
    fn recording_sink_last_event() {
        let mut sink = RecordingSink::<u32>::new();
        assert!(sink.last_event().is_none());

        sink.dispatch(1);
        assert_eq!(sink.last_event(), Some(&1));

        sink.dispatch(2);
        assert_eq!(sink.last_event(), Some(&2));
    }

    #[test]
    fn null_sink_discards_events() {
        let mut sink = NullSink;
        sink.dispatch(42);
        sink.dispatch(100);
        // 没有 panic 就是成功
    }
}
