#![no_main]

use libfuzzer_sys::fuzz_target;
use opentrace_bpf::utils::net::{ipaddr_to_u128, tcp_flags};

fuzz_target!(|data: &[u8]| {
    // 1. ipaddr_to_u128 — 字符串 IP 解析
    if let Ok(s) = std::str::from_utf8(data) {
        // 解析任意字符串为 IP，不能 panic
        let _ = ipaddr_to_u128(s);
    }

    // 2. tcp_flags — 任意 u16 生成 flag 字符串
    if data.len() >= 2 {
        let flags = u16::from_ne_bytes([data[0], data[1]]);
        let _ = tcp_flags(flags);
    }
});
