// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.

use std::fmt;

use serde::ser::SerializeStruct;
use serde::{Deserialize, Serialize};

use crate::protocol::{eth_proto, ip_proto};

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

#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct L2Info {
    pub eth_proto: u16,
}

#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct L3Info {
    pub saddr: Addr,
    pub daddr: Addr,
    pub tot_len: u16,
    pub ip_version: u16,
    pub l4_proto: u8,
}

#[derive(Clone, Copy, Debug)]
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
        let mut bytes = [0; 16];
        for (chunk, word) in bytes.chunks_exact_mut(4).zip(arr) {
            chunk.copy_from_slice(&word.to_ne_bytes());
        }

        Addr {
            v6addr: AddrV6 {
                upper: u64::from_ne_bytes(bytes[..8].try_into().unwrap()),
                lower: u64::from_ne_bytes(bytes[8..].try_into().unwrap()),
            },
        }
    }
}

impl fmt::Display for AddrV4 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}.{}.{}.{}",
            self.0 & 0xFF,
            (self.0 >> 8) & 0xFF,
            (self.0 >> 16) & 0xFF,
            (self.0 >> 24) & 0xFF
        )
    }
}

impl fmt::Display for AddrV6 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let upper = self.upper.to_ne_bytes();
        let lower = self.lower.to_ne_bytes();
        if upper == [0; 8] && lower[..4] == [0, 0, 0xff, 0xff] {
            return write!(f, "{}.{}.{}.{}", lower[4], lower[5], lower[6], lower[7]);
        }

        write!(
            f,
            "{:x}:{:x}:{:x}:{:x}:{:x}:{:x}:{:x}:{:x}",
            u16::from_be_bytes([upper[0], upper[1]]),
            u16::from_be_bytes([upper[2], upper[3]]),
            u16::from_be_bytes([upper[4], upper[5]]),
            u16::from_be_bytes([upper[6], upper[7]]),
            u16::from_be_bytes([lower[0], lower[1]]),
            u16::from_be_bytes([lower[2], lower[3]]),
            u16::from_be_bytes([lower[4], lower[5]]),
            u16::from_be_bytes([lower[6], lower[7]]),
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
    fn deserialize<D>(_deserializer: D) -> Result<Self, D::Error>
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
    fn deserialize<D>(_deserializer: D) -> Result<Self, D::Error>
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
    fn deserialize<D>(_deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        todo!()
    }
}

impl std::hash::Hash for Addr {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        unsafe {
            // 使用 v6addr 的内存表示进行哈希，覆盖 v4 和 v6 两种情况
            self.v6addr.upper.hash(state);
            self.v6addr.lower.hash(state);
        }
    }
}

impl PartialEq for Addr {
    fn eq(&self, other: &Self) -> bool {
        unsafe {
            self.v6addr.upper == other.v6addr.upper && self.v6addr.lower == other.v6addr.lower
        }
    }
}

impl Eq for Addr {}

impl fmt::Debug for Addr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        unsafe {
            // 默认显示为 v6addr，因为其内存布局覆盖全部 16 字节
            f.debug_struct("Addr")
                .field("v6addr", &self.v6addr)
                .finish()
        }
    }
}

impl fmt::Debug for AddrV6 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

#[cfg(test)]
mod tests {
    use super::{Addr, AddrV4, AddrV6, L3Info, L4Info};

    #[test]
    fn ipv6_display_uses_network_order_octets() {
        let addr = AddrV6 {
            upper: u64::from_ne_bytes([0x20, 0x01, 0x0d, 0xb8, 0x01, 0x02, 0x03, 0x04]),
            lower: u64::from_ne_bytes([0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c]),
        };

        assert_eq!(addr.to_string(), "2001:db8:102:304:506:708:90a:b0c");
    }

    #[test]
    fn ipv4_mapped_ipv6_displays_as_ipv4() {
        let addr = AddrV6 {
            upper: 0,
            lower: u64::from_ne_bytes([0, 0, 0xff, 0xff, 10, 253, 91, 214]),
        };

        assert_eq!(addr.to_string(), "10.253.91.214");
    }

    #[test]
    fn addr_from_u32_array_preserves_memory_order() {
        let addr = Addr::from([
            u32::from_ne_bytes([0x20, 0x01, 0x0d, 0xb8]),
            u32::from_ne_bytes([0x01, 0x02, 0x03, 0x04]),
            u32::from_ne_bytes([0x05, 0x06, 0x07, 0x08]),
            u32::from_ne_bytes([0x09, 0x0a, 0x0b, 0x0c]),
        ]);

        assert_eq!(
            AddrV6::from(addr).to_string(),
            "2001:db8:102:304:506:708:90a:b0c"
        );
    }

    #[test]
    fn ipv4_display_uses_network_order_bytes() {
        assert_eq!(
            AddrV4(u32::from_ne_bytes([192, 168, 1, 1])).to_string(),
            "192.168.1.1"
        );
    }

    #[test]
    fn l3_serialization_uses_ip_version_to_select_address_type() {
        let info = L3Info {
            saddr: Addr {
                v4addr: u32::from_ne_bytes([10, 0, 0, 1]),
            },
            daddr: Addr {
                v4addr: u32::from_ne_bytes([10, 0, 0, 2]),
            },
            tot_len: 0,
            ip_version: 4,
            l4_proto: 6,
        };

        let value = serde_json::to_value(info).unwrap();

        assert_eq!(value["src_ip"], "10.0.0.1");
        assert_eq!(value["dst_ip"], "10.0.0.2");
        assert_eq!(value["l3_proto"], "IPPROTO_TCP");
    }

    #[test]
    fn l4_serialization_converts_ports_from_network_order() {
        let info = L4Info {
            sport: u16::to_be(1234),
            dport: u16::to_be(80),
            tcpflags: 0,
        };

        let value = serde_json::to_value(info).unwrap();

        assert_eq!(value["sport"], 1234);
        assert_eq!(value["dport"], 80);
    }
}
