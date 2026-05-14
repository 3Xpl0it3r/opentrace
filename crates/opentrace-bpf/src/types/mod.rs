// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.

// 这个里面结构体都必须和src/bpf/侧定义的结构体在内存布局上保持一致
// 所以这个里面结构体必须都要加一个 `#[repr(C)]`

pub mod net;
pub mod process;
