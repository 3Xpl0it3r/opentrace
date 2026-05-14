// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.

use std::fmt;

use serde::ser::SerializeStruct;
use serde::{Deserialize, Serialize};

use crate::protocols::{eth_proto, ip_proto};

#[derive(Clone, Copy)]
#[repr(C)]
pub struct AddrV6 {
    pub upper: u64,
    pub lower: u64,
}

pub struct AddrV4(pub u32);

// 内存布局对应内核 union addr 结构体。
#[derive(Clone, Copy)]
#[repr(C)]
pub union Addr {
    pub v4addr: u32,
    pub v6addr: AddrV6,
}

#[derive(Clone, Copy)]
#[repr(C)]
pub struct L2Info {
    pub eth_proto: u16,
}

#[derive(Clone, Copy)]
#[repr(C)]
pub struct L3Info {
    pub saddr: Addr,
    pub daddr: Addr,
    pub tot_len: u16,
    pub ip_version: u16,
    pub l4_proto: u8,
}

#[derive(Clone, Copy)]
#[repr(C)]
pub struct L4Info {
    pub sport: u16,
    pub dport: u16,
    pub tcpflags: u16,
}

#[derive(Clone, Copy, Serialize, Deserialize)]
#[repr(C)]
pub struct PktInfo {
    pub ifname: [u8; 16],
    pub len: u32,
    pub cpu: u32,
    pub pid: u32,
    pub netns: u32,
    pub pkt_type: u8,
    pub _pad2: [u8; 3],
}

impl From<Addr> for AddrV4 {
    fn from(addr: Addr) -> Self {
        AddrV4(unsafe { addr.v4addr })
    }
}

impl From<Addr> for AddrV6 {
    fn from(addr: Addr) -> Self {
        unsafe { addr.v6addr }
    }
}

impl From<[u32; 4]> for Addr {
    fn from(arr: [u32; 4]) -> Self {
        Addr {
            v6addr: AddrV6 {
                upper: ((arr[0] as u64) << 32) | (arr[1] as u64),
                lower: ((arr[2] as u64) << 32) | (arr[3] as u64),
            },
        }
    }
}

impl fmt::Display for AddrV4 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}.{}.{}.{}",
            (self.0 >> 0) & 0xFF,
            (self.0 >> 8) & 0xFF,
            (self.0 >> 16) & 0xFF,
            (self.0 >> 24) & 0xFF
        )
    }
}


impl fmt::Display for AddrV6 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{:x}:{:x}:{:x}:{:x}:{:x}:{:x}:{:x}:{:x}",
            (self.upper >> 48) & 0xFFFF,
            (self.upper >> 32) & 0xFFFF,
            (self.upper >> 16) & 0xFFFF,
            (self.upper >> 0) & 0xFFFF,
            (self.lower >> 48) & 0xFFFF,
            (self.lower >> 32) & 0xFFFF,
            (self.lower >> 16) & 0xFFFF,
            (self.lower >> 0) & 0xFFFF,
        )
    }
}


impl Serialize for AddrV4 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.collect_str(self)
    }
}

impl Serialize for AddrV6 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.collect_str(self)
    }
}

impl Serialize for L2Info {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut state = serializer.serialize_struct("l2", 1)?;
        state.serialize_field("l2_proto", eth_proto::to_str(self.eth_proto))?;
        state.end()
    }
}

impl<'de> Deserialize<'de> for L2Info {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        todo!()
    }
}

impl Serialize for L3Info {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut state = serializer.serialize_struct("L3Info", 3)?;
        if self.ip_version == 4 {
            state.serialize_field("src_ip", &AddrV4::from(self.saddr))?;
            state.serialize_field("dst_ip", &AddrV4::from(self.daddr))?;
        } else {
            state.serialize_field("src_ip", &AddrV6::from(self.saddr))?;
            state.serialize_field("dst_ip", &AddrV6::from(self.daddr))?;
        }
        state.serialize_field("l3_proto", ip_proto::to_str(self.l4_proto as u16))?;
        state.end()
    }
}

impl<'de> Deserialize<'de> for L3Info {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        todo!()
    }
}

impl Serialize for L4Info {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut state = serializer.serialize_struct("L4Info", 2)?;
        state.serialize_field("dport", &u16::from_be(self.dport))?;
        state.serialize_field("sport", &u16::from_be(self.sport))?;
        state.end()
    }
}

impl<'de> Deserialize<'de> for L4Info {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        todo!()
    }
}
