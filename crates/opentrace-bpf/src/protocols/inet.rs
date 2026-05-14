// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.
use crate::EbpfError;

pub const IP: u16 = 0;
pub const ICMP: u16 = 1;
pub const IGMP: u16 = 2;
pub const IPIP: u16 = 4;
pub const TCP: u16 = 6;
pub const EGP: u16 = 8;
pub const PUP: u16 = 12;
pub const UDP: u16 = 17;
pub const IDP: u16 = 22;
pub const TP: u16 = 29;
pub const DCCP: u16 = 33;
pub const IPV6: u16 = 41;
pub const RSVP: u16 = 46;
pub const GRE: u16 = 47;
pub const ESP: u16 = 50;
pub const AH: u16 = 51;
pub const MTP: u16 = 92;
pub const BEETPH: u16 = 94;
pub const ENCAP: u16 = 98;
pub const PIM: u16 = 103;
pub const COMP: u16 = 108;
pub const SCTP: u16 = 132;
pub const UDPLITE: u16 = 136;
pub const MPLS: u16 = 137;
pub const ETHERNET: u16 = 143;
pub const RAW: u16 = 255;
pub const MPTCP: u16 = 262;
pub const MAX: u16 = 263;

pub fn to_str(n_proto: u16) -> &'static str {
    match n_proto {
        IP => "IPPROTO_IP",
        ICMP => "IPPROTO_ICMP",
        IGMP => "IPPROTO_IGMP",
        IPIP => "IPPROTO_IPIP",
        TCP => "IPPROTO_TCP",
        EGP => "IPPROTO_EGP",
        PUP => "IPPROTO_PUP",
        UDP => "IPPROTO_UDP",
        IDP => "IPPROTO_IDP",
        TP => "IPPROTO_TP",
        DCCP => "IPPROTO_DCCP",
        IPV6 => "IPPROTO_IPV6",
        RSVP => "IPPROTO_RSVP",
        GRE => "IPPROTO_GRE",
        ESP => "IPPROTO_ESP",
        AH => "IPPROTO_AH",
        MTP => "IPPROTO_MTP",
        BEETPH => "IPPROTO_BEETPH",
        ENCAP => "IPPROTO_ENCAP",
        PIM => "IPPROTO_PIM",
        COMP => "IPPROTO_COMP",
        SCTP => "IPPROTO_SCTP",
        UDPLITE => "IPPROTO_UDPLITE",
        MPLS => "IPPROTO_MPLS",
        ETHERNET => "IPPROTO_ETHERNET",
        RAW => "IPPROTO_RAW",
        MPTCP => "IPPROTO_MPTCP",
        MAX => "IPPROTO_MAX",
        _ => "UNKNOWN",
    }
}

pub fn parse(proto_name: &str) -> Result<u16, EbpfError> {
    match proto_name.to_lowercase().as_str() {
        "tcp" => Ok(TCP),
        "udp" => Ok(UDP),
        "icmp" => Ok(ICMP),
        _ => Err(EbpfError::Other(format!("UnSupport Proto: {}", proto_name))),
    }
}
