// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.

use std::ffi::CStr;

#[inline]
pub fn from_bytes_lossy(data: &[u8]) -> String {
    match CStr::from_bytes_until_nul(data) {
        Ok(value) => value.to_string_lossy().into_owned(),
        Err(_) => String::new(),
    }
}
