// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.

use serde::Serialize;

///  定义了内核发送出去的event数据如何被处理
///  - load: 内核到用户态这部分处理
///  - handle: 用户态到外部生态
pub trait Exporter<E: Send + Sized + Serialize + Clone> {
    // 可以用来处理event（序列化，转String, Folded格式化等等.....)
    // 也可以直接打印到终端，或者通过 channel发送出去
    fn handle(&mut self, event: E);

    // 不用自己覆盖, 直接无脑转换就行了（event和内核里面event内存布局是严格一一对应的,
    // 所以可以直接放心转换的_
    fn load(&self, data: &[u8]) -> E {
        unsafe { std::ptr::read(data.as_ptr() as *const E) }
    }
}
