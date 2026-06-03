// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.

mod stream_writer;
mod channel;
pub(crate) mod helper;

///  定义了内核发送出去的event数据如何被处理
///  - load: 内核到用户态这部分处理
///  - handle: 用户态到外部生态
pub trait EventSink<T> {
    fn load(&self, data: &[u8]) -> T {
        unsafe { std::ptr::read(data.as_ptr() as *const T) }
    }
    // 可以用来处理event（序列化，转String, Folded格式化等等.....)
    // 也可以直接打印到终端，或者通过 channel发送出去
    fn dispatch(&mut self, event: T);
}

pub use channel::BoundedChannelSink;
pub use channel::UnboundedChannelSink;
pub use stream_writer::StreamWriterSink;

#[cfg(test)]
mod tests {
    use super::EventSink;

    struct TestSink;

    impl EventSink<u32> for TestSink {
        fn dispatch(&mut self, _event: u32) {}
    }

    #[test]
    fn default_load_reads_plain_old_data() {
        let sink = TestSink;
        let bytes = 7_u32.to_ne_bytes();

        assert_eq!(sink.load(&bytes), 7);
    }
}
