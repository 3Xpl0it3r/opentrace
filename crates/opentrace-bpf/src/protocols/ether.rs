// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.

use crate::EbpfError;
use libc::htons;


pub const ETH_P_IP: u16 = htons(0x800);
pub const ETH_P_IPV6: u16 = htons(0x86dd);
pub const ETH_P_ARP: u16 = htons(0x0806);

pub fn to_str(n_proto: u16) -> &'static str {
    match n_proto {
        ETH_P_IP => "ETH_P_IP",
        ETH_P_IPV6 => "ETH_P_IPV6",
        ETH_P_ARP => "ETH_P_ARP",
        _ => "Unknown",
    }
}

pub fn parse(proto_name: &str) -> Result<u16, EbpfError> {
    match proto_name.to_lowercase().as_str() {
        "ip" | "ipv4" => Ok(ETH_P_IP),
        "ipv6" => Ok(ETH_P_IPV6),
        _ => Err(EbpfError::Other(format!(
            "UnSupport L2proto {}",
            proto_name
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::{ETH_P_IP, ETH_P_IPV6, parse, to_str};

    #[test]
    fn parses_supported_protocol_names_case_insensitively() {
        assert_eq!(parse("ip").unwrap(), ETH_P_IP);
        assert_eq!(parse("IPv4").unwrap(), ETH_P_IP);
        assert_eq!(parse("IPV6").unwrap(), ETH_P_IPV6);
    }

    #[test]
    fn rejects_unsupported_protocol_names() {
        assert!(parse("arp").is_err());
    }

    #[test]
    fn renders_protocol_names() {
        assert_eq!(to_str(ETH_P_IP), "ETH_P_IP");
        assert_eq!(to_str(ETH_P_IPV6), "ETH_P_IPV6");
        assert_eq!(to_str(0), "Unknown");
    }
}
