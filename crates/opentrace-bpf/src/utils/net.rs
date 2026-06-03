// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::str::FromStr as _;

use crate::EbpfError;

// 纯 Rust 实现的字节序转换，替代 libc 函数
#[inline]
const fn htonl(u: u32) -> u32 {
    u.to_be()
}

#[inline]
const fn ntohl(u: u32) -> u32 {
    u32::from_be(u)
}

#[inline]
pub fn u32_to_ipaddr_v4(addr: u32) -> String {
    Ipv4Addr::from(ntohl(addr)).to_string()
}

#[inline]
pub fn u128_to_ipaddr_v6(addr: u128) -> String {
    Ipv6Addr::from(addr).to_string()
}

// 因为Addr内存布局和内核union addr内存布局一致，所以这个地方要转化成大端存储
#[inline]
pub fn ipaddr_to_u128(ip_str: &str) -> Result<[u32; 4], EbpfError> {
    if ip_str.is_empty() {
        return Ok([0; 4]);
    }
    let ip = IpAddr::from_str(ip_str)?;
    match ip {
        IpAddr::V4(ipv4) => Ok([htonl(ipv4.to_bits()), 0, 0, 0]),
        IpAddr::V6(ipv6) => {
            let octets = ipv6.octets();
            Ok([
                u32::from_ne_bytes(octets[0..4].try_into().unwrap()),
                u32::from_ne_bytes(octets[4..8].try_into().unwrap()),
                u32::from_ne_bytes(octets[8..12].try_into().unwrap()),
                u32::from_ne_bytes(octets[12..16].try_into().unwrap()),
            ])
        }
    }
}

#[inline]
pub fn tcp_flags(flags: u16) -> String {
    let flags = u16::from_be(flags);
    let mut result = Vec::new();
    if flags & 0x01 != 0 {
        result.push("FIN");
    }
    if flags & 0x02 != 0 {
        result.push("SYN");
    }
    if flags & 0x04 != 0 {
        result.push("RST");
    }
    if flags & 0x08 != 0 {
        result.push("PSH");
    }
    if flags & 0x10 != 0 {
        result.push("ACK");
    }
    if flags & 0x20 != 0 {
        result.push("URG");
    }
    if result.is_empty() {
        "NONE".to_string()
    } else {
        result.join("-")
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::{ipaddr_to_u128, tcp_flags, u32_to_ipaddr_v4, u128_to_ipaddr_v6};

    #[test]
    fn formats_network_order_ipv4() {
        assert_eq!(
            u32_to_ipaddr_v4(u32::from_ne_bytes([127, 0, 0, 1])),
            "127.0.0.1"
        );
    }

    #[test]
    fn formats_ipv6_from_u128() {
        assert_eq!(u128_to_ipaddr_v6(1), "::1");
    }

    #[test]
    fn empty_filter_address_converts_to_zero_words() {
        assert_eq!(ipaddr_to_u128("").unwrap(), [0; 4]);
    }

    #[test]
    fn ipv4_filter_address_uses_first_word_only() {
        let words = ipaddr_to_u128("127.0.0.1").unwrap();
        assert_eq!(words[0].to_ne_bytes(), [127, 0, 0, 1]);
        assert_eq!(&words[1..], &[0, 0, 0]);
    }

    #[test]
    fn ipv6_conversion_preserves_network_order_octets() {
        let words = ipaddr_to_u128("2001:db8:102:304:506:708:90a:b0c").unwrap();
        let mut bytes = [0; 16];
        for (chunk, word) in bytes.chunks_exact_mut(4).zip(words) {
            chunk.copy_from_slice(&word.to_ne_bytes());
        }

        assert_eq!(
            bytes,
            [
                0x20, 0x01, 0x0d, 0xb8, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a,
                0x0b, 0x0c,
            ]
        );
    }

    #[test]
    fn formats_tcp_flags() {
        assert_eq!(tcp_flags(u16::to_be(0x12)), "SYN-ACK");
        assert_eq!(tcp_flags(0), "NONE");
    }

    #[rstest]
    #[case(u16::to_be(0x01), "FIN")]
    #[case(u16::to_be(0x02), "SYN")]
    #[case(u16::to_be(0x04), "RST")]
    #[case(u16::to_be(0x08), "PSH")]
    #[case(u16::to_be(0x10), "ACK")]
    #[case(u16::to_be(0x20), "URG")]
    #[case(u16::to_be(0x1f), "FIN-SYN-RST-PSH-ACK")]
    #[case(u16::to_be(0x3f), "FIN-SYN-RST-PSH-ACK-URG")]
    fn tcp_flags_individual_and_combined(#[case] input: u16, #[case] expected: &str) {
        assert_eq!(tcp_flags(input), expected);
    }

    #[rstest]
    #[case("not.an.ip")]
    #[case("1.2.3.4.5")]
    #[case("999.999.999.999")]
    #[case("::g")]
    #[case("fe80:")]
    fn ipaddr_to_u128_rejects_invalid_inputs(#[case] input: &str) {
        assert!(ipaddr_to_u128(input).is_err());
    }
}
