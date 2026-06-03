#![no_main]

use libfuzzer_sys::fuzz_target;
use opentrace_bpf::collectors::net::SkbdropEvent;
use opentrace_bpf::types::net::{Addr, L2Info, L3Info, L4Info};

fuzz_target!(|data: &[u8]| {
    if data.len() < 32 {
        return;
    }

    let drop_source = data[0];
    let drop_reason = data[1];

    let saddr = Addr {
        v4addr: u32::from_ne_bytes([data[2], data[3], data[4], data[5]]),
    };
    let daddr = Addr {
        v4addr: u32::from_ne_bytes([data[6], data[7], data[8], data[9]]),
    };

    let sport = u16::from_ne_bytes([data[10], data[11]]);
    let dport = u16::from_ne_bytes([data[12], data[13]]);
    let tcpflags = u16::from_ne_bytes([data[14], data[15]]);

    let ip_version = u16::from_ne_bytes([data[16], data[17]]);
    let l4_proto = data[18];
    let eth_proto = u16::from_ne_bytes([data[19], data[20]]);

    // stack_size: 使用 i64，覆盖负/零/正
    let stack_size_bytes = [
        data[21], data[22], data[23], data[24], data[25], data[26], data[27], data[28],
    ];
    let stack_size = i64::from_ne_bytes(stack_size_bytes);

    // 取 2 个 u64 作为栈数据
    let stack0 = u64::from_ne_bytes([data[29], data[30], data[31], 0, 0, 0, 0, 0]);
    let stack1 = if data.len() >= 40 {
        u64::from_ne_bytes([
            data[32], data[33], data[34], data[35], data[36], data[37], data[38], data[39],
        ])
    } else {
        0
    };

    let mut stack = [0u64; 16];
    stack[0] = stack0;
    stack[1] = stack1;

    let event = SkbdropEvent {
        l2_info: L2Info { eth_proto },
        l3_info: L3Info {
            saddr,
            daddr,
            tot_len: 0,
            ip_version,
            l4_proto,
        },
        l4_info: L4Info {
            sport,
            dport,
            tcpflags,
        },
        stack_size,
        stack,
        drop_reason,
        drop_source,
    };

    // 序列化不能 panic — 最关键的不变式
    let json = serde_json::to_string(&event);
    let _ = json;

    // to_value 也应该成功
    let _ = serde_json::to_value(event);
});
