// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.

pub mod eth_proto {
    use libc::htons;

    use crate::EbpfError;

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
}

pub mod ip_proto {
    use crate::EbpfError;

    pub const IPPROTO_IP: u16 = 0;
    pub const IPPROTO_ICMP: u16 = 1;
    pub const IPPROTO_IGMP: u16 = 2;
    pub const IPPROTO_IPIP: u16 = 4;
    pub const IPPROTO_TCP: u16 = 6;
    pub const IPPROTO_EGP: u16 = 8;
    pub const IPPROTO_PUP: u16 = 12;
    pub const IPPROTO_UDP: u16 = 17;
    pub const IPPROTO_IDP: u16 = 22;
    pub const IPPROTO_TP: u16 = 29;
    pub const IPPROTO_DCCP: u16 = 33;
    pub const IPPROTO_IPV6: u16 = 41;
    pub const IPPROTO_RSVP: u16 = 46;
    pub const IPPROTO_GRE: u16 = 47;
    pub const IPPROTO_ESP: u16 = 50;
    pub const IPPROTO_AH: u16 = 51;
    pub const IPPROTO_MTP: u16 = 92;
    pub const IPPROTO_BEETPH: u16 = 94;
    pub const IPPROTO_ENCAP: u16 = 98;
    pub const IPPROTO_PIM: u16 = 103;
    pub const IPPROTO_COMP: u16 = 108;
    pub const IPPROTO_SCTP: u16 = 132;
    pub const IPPROTO_UDPLITE: u16 = 136;
    pub const IPPROTO_MPLS: u16 = 137;
    pub const IPPROTO_ETHERNET: u16 = 143;
    pub const IPPROTO_RAW: u16 = 255;
    pub const IPPROTO_MPTCP: u16 = 262;
    pub const IPPROTO_MAX: u16 = 263;

    pub fn to_str(n_proto: u16) -> &'static str {
        match n_proto {
            IPPROTO_IP => "IPPROTO_IP",
            IPPROTO_ICMP => "IPPROTO_ICMP",
            IPPROTO_IGMP => "IPPROTO_IGMP",
            IPPROTO_IPIP => "IPPROTO_IPIP",
            IPPROTO_TCP => "IPPROTO_TCP",
            IPPROTO_EGP => "IPPROTO_EGP",
            IPPROTO_PUP => "IPPROTO_PUP",
            IPPROTO_UDP => "IPPROTO_UDP",
            IPPROTO_IDP => "IPPROTO_IDP",
            IPPROTO_TP => "IPPROTO_TP",
            IPPROTO_DCCP => "IPPROTO_DCCP",
            IPPROTO_IPV6 => "IPPROTO_IPV6",
            IPPROTO_RSVP => "IPPROTO_RSVP",
            IPPROTO_GRE => "IPPROTO_GRE",
            IPPROTO_ESP => "IPPROTO_ESP",
            IPPROTO_AH => "IPPROTO_AH",
            IPPROTO_MTP => "IPPROTO_MTP",
            IPPROTO_BEETPH => "IPPROTO_BEETPH",
            IPPROTO_ENCAP => "IPPROTO_ENCAP",
            IPPROTO_PIM => "IPPROTO_PIM",
            IPPROTO_COMP => "IPPROTO_COMP",
            IPPROTO_SCTP => "IPPROTO_SCTP",
            IPPROTO_UDPLITE => "IPPROTO_UDPLITE",
            IPPROTO_MPLS => "IPPROTO_MPLS",
            IPPROTO_ETHERNET => "IPPROTO_ETHERNET",
            IPPROTO_RAW => "IPPROTO_RAW",
            IPPROTO_MPTCP => "IPPROTO_MPTCP",
            IPPROTO_MAX => "IPPROTO_MAX",
            _ => "UNKNOWN",
        }
    }

    pub fn parse(proto_name: &str) -> Result<u16, EbpfError> {
        match proto_name.to_lowercase().as_str() {
            "tcp" => Ok(IPPROTO_TCP),
            "udp" => Ok(IPPROTO_UDP),
            "icmp" => Ok(IPPROTO_ICMP),
            _ => Err(EbpfError::Other(format!("UnSupport Proto: {}", proto_name))),
        }
    }
}
