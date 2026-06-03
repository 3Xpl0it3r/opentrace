// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.
use std::ffi::CString;
use std::mem::MaybeUninit;

// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.
use libbpf_rs::{OpenObject, libbpf_sys};

use crate::EbpfError;

pub type CollectorObject = MaybeUninit<OpenObject>;

pub fn open_object_storage() -> CollectorObject {
    MaybeUninit::uninit()
}

pub(crate) fn with_custom_btf_open_opts<T>(
    btf_cus_path: &str,
    open: impl FnOnce(libbpf_sys::bpf_object_open_opts) -> Result<T, EbpfError>,
) -> Result<T, EbpfError> {
    let path = CString::new(btf_cus_path)
        .map_err(|e| EbpfError::Other(format!("Custom Btfpath to cstring failed {}", e)))?;

    let mut opts = libbpf_sys::bpf_object_open_opts {
        sz: std::mem::size_of::<libbpf_sys::bpf_object_open_opts>() as libbpf_sys::size_t,
        ..Default::default()
    };
    opts.btf_custom_path = path.as_ptr();
    open(opts)
}

#[cfg(test)]
mod tests {
    use super::with_custom_btf_open_opts;

    #[test]
    fn rejects_btf_path_with_interior_nul() {
        let result = with_custom_btf_open_opts("bad\0path", |_| Ok(()));

        assert!(result.is_err());
    }

    #[test]
    fn passes_custom_btf_path_to_open_callback() {
        let result = with_custom_btf_open_opts("/tmp/vmlinux.btf", |opts| {
            assert!(!opts.btf_custom_path.is_null());
            Ok(42)
        })
        .unwrap();

        assert_eq!(result, 42);
    }
}
