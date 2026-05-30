// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.
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
