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
