// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.

use std::ffi::CStr;

#[inline]
pub fn from_bytes_lossy(data: &[u8]) -> String {
    match CStr::from_bytes_until_nul(data) {
        Ok(value) => value.to_string_lossy().into_owned(),
        Err(_) => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::from_bytes_lossy;

    #[test]
    fn reads_until_first_nul() {
        assert_eq!(from_bytes_lossy(b"eth0\0ignored"), "eth0");
    }

    #[test]
    fn returns_empty_when_no_nul_is_present() {
        assert_eq!(from_bytes_lossy(b"eth0"), "");
    }

    #[test]
    fn replaces_invalid_utf8_lossily() {
        assert_eq!(from_bytes_lossy(&[0xff, 0x00]), "\u{fffd}");
    }
}
