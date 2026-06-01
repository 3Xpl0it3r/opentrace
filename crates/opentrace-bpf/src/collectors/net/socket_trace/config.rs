use std::{mem, slice};

// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.
pub struct Config {
    pub custom_btf_path: Option<String>,
    pub pid: u32,
    pub verbose: bool,
}

impl From<Config> for InnerConfig {
    fn from(value: Config) -> Self {
        InnerConfig { pid: value.pid }
    }
}

#[repr(C)]
pub(super) struct InnerConfig {
    pid: u32,
}

impl InnerConfig {
    pub(super) fn as_bytes(&self) -> &[u8] {
        let ptr = self as *const Self as *const u8;
        let len = mem::size_of_val(self);
        unsafe { slice::from_raw_parts(ptr, len) }
    }
}
