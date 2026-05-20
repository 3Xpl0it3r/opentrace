// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.
// formatter主要用于opentrace-bpf 侧event的格式化转化, 转换成其他crate需要的格式
// 做初次加工

use std::io::{self, Write};

pub trait StreamFormatter<T> {
    fn format<W: Write>(&self, w: &mut W, args: &T) -> io::Result<()>;
}

// 用于结构化输出（其实就是类似into trait)
pub trait StructeredFormatter<E: Sized> {
    type Output;
    fn format(&self, from: E) -> Result<Self::Output, io::Error>;
}

// JsonFormatter<T>[#TODO] (shoule add some comments )
