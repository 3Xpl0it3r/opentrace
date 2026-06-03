#![no_main]

use libfuzzer_sys::fuzz_target;
use opentrace_bpf::utils::cstring::from_bytes_lossy;
use opentrace_bpf::utils::net::{ipaddr_to_u128, tcp_flags};

fuzz_target!(|data: &[u8]| {
    // 1. from_bytes_lossy — 从任意字节序列中提取 C 字符串
    let s = from_bytes_lossy(data);
    assert!(!s.contains('\0'));
    let _ = s.len();
    let _ = s.is_empty();

    // 2. ipaddr_to_u128 — 字符串 IP 解析
    if let Ok(s) = std::str::from_utf8(data) {
        let _ = ipaddr_to_u128(s);
    }

    // 3. tcp_flags — 任意 u16 生成 flag 字符串
    if data.len() >= 2 {
        let flags = u16::from_ne_bytes([data[0], data[1]]);
        let s = tcp_flags(flags);
        assert!(!s.is_empty());
    }

    // 4. 测试边界情况
    if data.len() >= 4 {
        // 测试 IPv4 地址解析
        let addr = format!("{}.{}.{}.{}", data[0], data[1], data[2], data[3]);
        let _ = ipaddr_to_u128(&addr);
    }

    // 5. 测试 IPv6 地址格式
    if data.len() >= 16 {
        let addr = format!(
            "{:x}:{:x}:{:x}:{:x}:{:x}:{:x}:{:x}:{:x}",
            u16::from_ne_bytes([data[0], data[1]]),
            u16::from_ne_bytes([data[2], data[3]]),
            u16::from_ne_bytes([data[4], data[5]]),
            u16::from_ne_bytes([data[6], data[7]]),
            u16::from_ne_bytes([data[8], data[9]]),
            u16::from_ne_bytes([data[10], data[11]]),
            u16::from_ne_bytes([data[12], data[13]]),
            u16::from_ne_bytes([data[14], data[15]]),
        );
        let _ = ipaddr_to_u128(&addr);
    }

    // 6. 测试各种 TCP flag 组合
    if data.len() >= 2 {
        let flags = u16::from_ne_bytes([data[0], data[1]]);
        let s = tcp_flags(flags);

        // 验证输出格式
        if s == "NONE" {
            assert_eq!(flags, 0);
        } else {
            assert!(s.contains("FIN") || s.contains("SYN") || s.contains("RST") ||
                    s.contains("PSH") || s.contains("ACK") || s.contains("URG"));
        }
    }
});
