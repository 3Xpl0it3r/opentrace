#![no_main]

use libfuzzer_sys::fuzz_target;
use opentrace_bpf::protocol::{eth_proto, ip_proto};

fuzz_target!(|data: &[u8]| {
    // 尝试将数据解释为协议名称字符串
    if let Ok(s) = std::str::from_utf8(data) {
        // Fuzz 以太网协议解析
        let _ = eth_proto::parse(s);

        // Fuzz IP 协议解析
        let _ = ip_proto::parse(s);
    }

    // 测试原始字节作为协议号
    if data.len() >= 2 {
        let proto = u16::from_ne_bytes([data[0], data[1]]);
        let _ = eth_proto::to_str(proto);
        let _ = ip_proto::to_str(proto);
    }
});
