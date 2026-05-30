// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.

mod stdout;
mod channel;
pub(crate) mod helper;

///  定义了内核发送出去的event数据如何被处理
///  - load: 内核到用户态这部分处理
///  - handle: 用户态到外部生态
pub trait Exporter<T> {
    fn load(&self, data: &[u8]) -> T {
        unsafe { std::ptr::read(data.as_ptr() as *const T) }
    }
    // 可以用来处理event（序列化，转String, Folded格式化等等.....)
    // 也可以直接打印到终端，或者通过 channel发送出去
    fn dispatch(&mut self, event: T);
}

pub use channel::SimpleBoundChannelExpoter;
pub use channel::SimpleUnboundChannelExporter;
pub use stdout::DefaultStdoutExporter;
