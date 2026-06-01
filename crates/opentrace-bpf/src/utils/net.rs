// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::str::FromStr as _;

use libc::{htonl, ntohl};

use crate::EbpfError;

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

#[allow(dead_code)]
#[inline]
pub(super) fn tcp_flags(flags: u16) -> String {
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
    use super::ipaddr_to_u128;

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
}
