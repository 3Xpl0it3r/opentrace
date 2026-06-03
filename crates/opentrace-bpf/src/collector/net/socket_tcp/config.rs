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

#[cfg(test)]
mod tests {
    use super::{Config, InnerConfig};

    #[test]
    fn converts_public_config_to_inner_config() {
        let inner = InnerConfig::from(Config {
            custom_btf_path: Some("btf".into()),
            pid: 42,
            verbose: true,
        });

        assert_eq!(inner.pid, 42);
    }

    #[test]
    fn exposes_inner_config_as_struct_bytes() {
        let inner = InnerConfig { pid: 42 };

        assert_eq!(inner.as_bytes(), &42_u32.to_ne_bytes());
    }
}
