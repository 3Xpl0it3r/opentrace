#![no_main]

use libfuzzer_sys::fuzz_target;
use opentrace_bpf::types::process::ProcessInfo;

fuzz_target!(|data: &[u8]| {
    // 1. 测试 ProcessInfo 序列化
    if data.len() >= 20 {
        let pid = u32::from_ne_bytes([data[0], data[1], data[2], data[3]]);
        let tgid = u32::from_ne_bytes([data[4], data[5], data[6], data[7]]);

        let mut comm = [0u8; 16];
        comm.copy_from_slice(&data[8..24]);

        let process = ProcessInfo { pid, tgid, comm };

        // 序列化不能 panic
        let json = serde_json::to_string(&process);
        assert!(json.is_ok());

        // to_value 也应该成功
        let value = serde_json::to_value(process);
        assert!(value.is_ok());
    }

    // 2. 测试空 comm
    if data.len() >= 8 {
        let pid = u32::from_ne_bytes([data[0], data[1], data[2], data[3]]);
        let tgid = u32::from_ne_bytes([data[4], data[5], data[6], data[7]]);

        let process = ProcessInfo {
            pid,
            tgid,
            comm: [0; 16],
        };

        let json = serde_json::to_string(&process);
        assert!(json.is_ok());
    }

    // 3. 测试 comm 包含 nul 字符
    if data.len() >= 8 {
        let pid = u32::from_ne_bytes([data[0], data[1], data[2], data[3]]);
        let tgid = u32::from_ne_bytes([data[4], data[5], data[6], data[7]]);

        let mut comm = [0u8; 16];
        comm[0] = b't';
        comm[1] = b'e';
        comm[2] = b's';
        comm[3] = b't';
        comm[4] = 0; // nul 字符
        comm[5] = b'x';

        let process = ProcessInfo { pid, tgid, comm };

        let json = serde_json::to_string(&process);
        assert!(json.is_ok());
    }
});
