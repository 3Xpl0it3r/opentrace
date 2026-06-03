// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.

//! 集成测试：types 模块
//!
//! 测试网络类型和进程类型

use opentrace_bpf::types::net::{Addr, AddrV4, AddrV6, L2Info, L3Info, L4Info};
use opentrace_bpf::types::process::ProcessInfo;
use rstest::rstest;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

// ==================== AddrV4 测试 ====================

#[rstest]
#[case(u32::from_ne_bytes([192, 168, 1, 1]), "192.168.1.1")]
#[case(u32::from_ne_bytes([10, 0, 0, 1]), "10.0.0.1")]
#[case(u32::from_ne_bytes([127, 0, 0, 1]), "127.0.0.1")]
#[case(u32::from_ne_bytes([0, 0, 0, 0]), "0.0.0.0")]
fn addr_v4_display(#[case] addr: u32, #[case] expected: &str) {
    assert_eq!(AddrV4(addr).to_string(), expected);
}

#[test]
fn addr_v4_from_addr() {
    let addr = Addr { v4addr: u32::from_ne_bytes([192, 168, 1, 1]) };
    let v4 = AddrV4::from(addr);
    assert_eq!(v4.to_string(), "192.168.1.1");
}

// ==================== AddrV6 测试 ====================

#[test]
fn addr_v6_display_full() {
    let addr = AddrV6 {
        upper: u64::from_ne_bytes([0x20, 0x01, 0x0d, 0xb8, 0x01, 0x02, 0x03, 0x04]),
        lower: u64::from_ne_bytes([0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c]),
    };
    assert_eq!(addr.to_string(), "2001:db8:102:304:506:708:90a:b0c");
}

#[test]
fn addr_v6_display_ipv4_mapped() {
    let addr = AddrV6 {
        upper: 0,
        lower: u64::from_ne_bytes([0, 0, 0xff, 0xff, 10, 253, 91, 214]),
    };
    assert_eq!(addr.to_string(), "10.253.91.214");
}

// ==================== Addr union 测试 ====================

#[test]
fn addr_from_u32_array() {
    let arr = [
        u32::from_ne_bytes([0x20, 0x01, 0x0d, 0xb8]),
        u32::from_ne_bytes([0x01, 0x02, 0x03, 0x04]),
        u32::from_ne_bytes([0x05, 0x06, 0x07, 0x08]),
        u32::from_ne_bytes([0x09, 0x0a, 0x0b, 0x0c]),
    ];
    let addr = Addr::from(arr);
    let v6 = AddrV6::from(addr);
    assert_eq!(v6.to_string(), "2001:db8:102:304:506:708:90a:b0c");
}

#[test]
fn addr_hash_and_eq() {
    let addr1 = Addr { v4addr: u32::from_ne_bytes([10, 0, 0, 1]) };
    let addr2 = Addr { v4addr: u32::from_ne_bytes([10, 0, 0, 1]) };
    let addr3 = Addr { v4addr: u32::from_ne_bytes([10, 0, 0, 2]) };

    assert_eq!(addr1, addr2);
    assert_ne!(addr1, addr3);

    let mut h1 = DefaultHasher::new();
    let mut h2 = DefaultHasher::new();
    addr1.hash(&mut h1);
    addr2.hash(&mut h2);
    assert_eq!(h1.finish(), h2.finish());
}

// ==================== L2Info 测试 ====================

#[test]
fn l2_info_serialization() {
    let l2 = L2Info { eth_proto: 0x0800 };
    let json = serde_json::to_string(&l2).unwrap();
    assert!(json.contains("l2_proto"));
}

// ==================== L3Info 测试 ====================

#[test]
fn l3_info_serialization_ipv4() {
    let l3 = L3Info {
        saddr: Addr { v4addr: u32::from_ne_bytes([10, 0, 0, 1]) },
        daddr: Addr { v4addr: u32::from_ne_bytes([10, 0, 0, 2]) },
        tot_len: 1500,
        ip_version: 4,
        l4_proto: 6,
    };
    let json = serde_json::to_string(&l3).unwrap();
    assert!(json.contains("src_ip"));
    assert!(json.contains("dst_ip"));
    assert!(json.contains("l3_proto"));
}

#[test]
fn l3_info_serialization_ipv6() {
    let l3 = L3Info {
        saddr: Addr {
            v6addr: AddrV6 { upper: 0, lower: 1 },
        },
        daddr: Addr {
            v6addr: AddrV6 { upper: 0, lower: 2 },
        },
        tot_len: 1500,
        ip_version: 6,
        l4_proto: 6,
    };
    let json = serde_json::to_string(&l3).unwrap();
    assert!(json.contains("src_ip"));
    assert!(json.contains("dst_ip"));
}

// ==================== L4Info 测试 ====================

#[test]
fn l4_info_serialization() {
    let l4 = L4Info {
        sport: u16::to_be(1234),
        dport: u16::to_be(80),
        tcpflags: u16::to_be(0x12),
    };
    let json = serde_json::to_string(&l4).unwrap();
    assert!(json.contains("sport"));
    assert!(json.contains("dport"));
}

// ==================== ProcessInfo 测试 ====================

#[test]
fn process_info_serialization() {
    let process = ProcessInfo {
        pid: 1234,
        tgid: 1234,
        comm: {
            let mut comm = [0u8; 16];
            comm[..6].copy_from_slice(b"myapp\0");
            comm
        },
    };
    let json = serde_json::to_string(&process).unwrap();
    assert!(json.contains("1234"));
    assert!(json.contains("myapp"));
}

#[test]
fn process_info_serialization_with_nul_in_comm() {
    let process = ProcessInfo {
        pid: 1,
        tgid: 1,
        comm: {
            let mut comm = [0u8; 16];
            comm[0] = b't';
            comm[1] = b'e';
            comm[2] = b's';
            comm[3] = b't';
            comm[4] = 0; // nul 字符
            comm[5] = b'x';
            comm
        },
    };
    let json = serde_json::to_string(&process).unwrap();
    assert!(json.contains("test"));
    // nul 之后的字符不应该出现
    assert!(!json.contains("testx"));
}
