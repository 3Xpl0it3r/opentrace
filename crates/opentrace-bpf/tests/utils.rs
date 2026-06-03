// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.

//! 集成测试：utils 模块
//!
//! 测试工具函数

use opentrace_bpf::utils::cstring::from_bytes_lossy;
use opentrace_bpf::utils::net::{ipaddr_to_u128, tcp_flags, u32_to_ipaddr_v4, u128_to_ipaddr_v6};
use opentrace_bpf::utils::units::bytes::Bytes;
use opentrace_bpf::utils::units::time::{Nanoseconds, TimeAsNanosecond};
use rstest::rstest;

// ==================== bytes 测试 ====================

#[rstest]
#[case(0, "0B")]
#[case(1023, "1023B")]
#[case(1024, "1k")]
#[case(1536, "1k")]
#[case(1024 * 1024, "1M")]
#[case(2 * 1024 * 1024 + 512, "2M")]
fn bytes_display(#[case] input: u32, #[case] expected: &str) {
    assert_eq!(Bytes(input).to_string(), expected);
}

// ==================== cstring 测试 ====================

#[test]
fn cstring_from_bytes_lossy_with_nul() {
    assert_eq!(from_bytes_lossy(b"eth0\0ignored"), "eth0");
}

#[test]
fn cstring_from_bytes_lossy_without_nul() {
    assert_eq!(from_bytes_lossy(b"eth0"), "");
}

#[test]
fn cstring_from_bytes_lossy_invalid_utf8() {
    assert_eq!(from_bytes_lossy(&[0xff, 0x00]), "\u{fffd}");
}

#[test]
fn cstring_from_bytes_lossy_empty() {
    assert_eq!(from_bytes_lossy(b""), "");
}

// ==================== net 测试 ====================

#[test]
fn u32_to_ipaddr_v4_basic() {
    assert_eq!(
        u32_to_ipaddr_v4(u32::from_ne_bytes([127, 0, 0, 1])),
        "127.0.0.1"
    );
}

#[test]
fn u128_to_ipaddr_v6_loopback() {
    assert_eq!(u128_to_ipaddr_v6(1), "::1");
}

#[test]
fn ipaddr_to_u128_empty() {
    assert_eq!(ipaddr_to_u128("").unwrap(), [0; 4]);
}

#[test]
fn ipaddr_to_u128_ipv4() {
    let words = ipaddr_to_u128("127.0.0.1").unwrap();
    assert_eq!(words[0].to_ne_bytes(), [127, 0, 0, 1]);
    assert_eq!(&words[1..], &[0, 0, 0]);
}

#[test]
fn ipaddr_to_u128_ipv6() {
    let words = ipaddr_to_u128("2001:db8:102:304:506:708:90a:b0c").unwrap();
    let mut bytes = [0; 16];
    for (chunk, word) in bytes.chunks_exact_mut(4).zip(words) {
        chunk.copy_from_slice(&word.to_ne_bytes());
    }
    assert_eq!(
        bytes,
        [
            0x20, 0x01, 0x0d, 0xb8, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a,
            0x0b, 0x0c
        ]
    );
}

#[rstest]
#[case("not.an.ip")]
#[case("1.2.3.4.5")]
#[case("999.999.999.999")]
#[case("::g")]
#[case("fe80:")]
fn ipaddr_to_u128_rejects_invalid(#[case] input: &str) {
    assert!(ipaddr_to_u128(input).is_err());
}

#[rstest]
#[case(u16::to_be(0x01), "FIN")]
#[case(u16::to_be(0x02), "SYN")]
#[case(u16::to_be(0x04), "RST")]
#[case(u16::to_be(0x08), "PSH")]
#[case(u16::to_be(0x10), "ACK")]
#[case(u16::to_be(0x20), "URG")]
#[case(u16::to_be(0x12), "SYN-ACK")]
#[case(0, "NONE")]
fn tcp_flags_display(#[case] input: u16, #[case] expected: &str) {
    assert_eq!(tcp_flags(input), expected);
}

// ==================== time 测试 ====================

#[test]
fn time_as_nanosecond_zero() {
    assert_eq!(TimeAsNanosecond(0).to_string(), "00:00:00");
}

#[test]
fn time_as_nanosecond_seconds() {
    assert_eq!(TimeAsNanosecond(1_000_000_000).to_string(), "00:00:01");
}

#[test]
fn time_as_nanosecond_milliseconds() {
    assert_eq!(TimeAsNanosecond(1_234_567_890).to_string(), "00:00:01.234");
}

#[test]
fn time_as_nanosecond_hours() {
    assert_eq!(TimeAsNanosecond(3_723_000_000_000).to_string(), "01:02:03");
}

#[rstest]
#[case(999, "999ns")]
#[case(1_000, "1us")]
#[case(1_999, "1us")]
#[case(1_000_000, "1ms")]
#[case(1_999_999, "1ms")]
#[case(1_000_000_000, "1s")]
fn nanoseconds_display(#[case] input: u64, #[case] expected: &str) {
    assert_eq!(Nanoseconds(input).to_string(), expected);
}
