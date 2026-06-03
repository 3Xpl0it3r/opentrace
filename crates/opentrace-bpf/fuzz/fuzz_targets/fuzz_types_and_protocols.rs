#![no_main]

use libfuzzer_sys::fuzz_target;
use opentrace_bpf::protocols::{eth_proto, ip_proto};
use opentrace_bpf::types::net::{Addr, AddrV4, AddrV6, L2Info, L3Info, L4Info};

fuzz_target!(|data: &[u8]| {
    // 1. 测试 AddrV4 Display
    if data.len() >= 4 {
        let addr = AddrV4(u32::from_ne_bytes([data[0], data[1], data[2], data[3]]));
        let s = addr.to_string();
        assert!(!s.is_empty());
    }

    // 2. 测试 AddrV6 Display
    if data.len() >= 16 {
        let upper = u64::from_ne_bytes([
            data[0], data[1], data[2], data[3], data[4], data[5], data[6], data[7],
        ]);
        let lower = u64::from_ne_bytes([
            data[8], data[9], data[10], data[11], data[12], data[13], data[14], data[15],
        ]);
        let addr = AddrV6 { upper, lower };
        let s = addr.to_string();
        assert!(!s.is_empty());
    }

    // 3. 测试 Addr union
    if data.len() >= 16 {
        let addr = Addr {
            v6addr: AddrV6 {
                upper: u64::from_ne_bytes([
                    data[0], data[1], data[2], data[3], data[4], data[5], data[6], data[7],
                ]),
                lower: u64::from_ne_bytes([
                    data[8], data[9], data[10], data[11], data[12], data[13], data[14], data[15],
                ]),
            },
        };

        // 测试 From trait
        let v4 = AddrV4::from(addr);
        let _ = v4.to_string();

        let v6 = AddrV6::from(addr);
        let _ = v6.to_string();

        // 测试 Hash 和 PartialEq
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        addr.hash(&mut hasher);
        let _ = hasher.finish();

        // 比较两个相同的地址
        let addr2 = Addr {
            v6addr: AddrV6 {
                upper: u64::from_ne_bytes([
                    data[0], data[1], data[2], data[3], data[4], data[5], data[6], data[7],
                ]),
                lower: u64::from_ne_bytes([
                    data[8], data[9], data[10], data[11], data[12], data[13], data[14], data[15],
                ]),
            },
        };
        assert_eq!(addr, addr2);
    }

    // 4. 测试 L2Info 序列化
    if data.len() >= 2 {
        let l2 = L2Info {
            eth_proto: u16::from_ne_bytes([data[0], data[1]]),
        };
        let _ = serde_json::to_string(&l2);
    }

    // 5. 测试 L3Info 序列化
    if data.len() >= 18 {
        let l3 = L3Info {
            saddr: Addr {
                v4addr: u32::from_ne_bytes([data[0], data[1], data[2], data[3]]),
            },
            daddr: Addr {
                v4addr: u32::from_ne_bytes([data[4], data[5], data[6], data[7]]),
            },
            tot_len: u16::from_ne_bytes([data[8], data[9]]),
            ip_version: data[10] as u16,
            l4_proto: data[11],
        };
        let _ = serde_json::to_string(&l3);
    }

    // 6. 测试 L4Info 序列化
    if data.len() >= 6 {
        let l4 = L4Info {
            sport: u16::from_ne_bytes([data[0], data[1]]),
            dport: u16::from_ne_bytes([data[2], data[3]]),
            tcpflags: u16::from_ne_bytes([data[4], data[5]]),
        };
        let _ = serde_json::to_string(&l4);
    }

    // 7. 测试 From<[u32; 4]>
    if data.len() >= 16 {
        let arr = [
            u32::from_ne_bytes([data[0], data[1], data[2], data[3]]),
            u32::from_ne_bytes([data[4], data[5], data[6], data[7]]),
            u32::from_ne_bytes([data[8], data[9], data[10], data[11]]),
            u32::from_ne_bytes([data[12], data[13], data[14], data[15]]),
        ];
        let addr = Addr::from(arr);
        let v6 = AddrV6::from(addr);
        let _ = v6.to_string();
    }

    // 8. 测试协议解析
    if let Ok(s) = std::str::from_utf8(data) {
        let _ = eth_proto::parse(s);
        let _ = ip_proto::parse(s);
    }

    // 9. 测试协议号到字符串
    if data.len() >= 2 {
        let proto = u16::from_ne_bytes([data[0], data[1]]);
        let _ = eth_proto::to_str(proto);
        let _ = ip_proto::to_str(proto);
    }
});
