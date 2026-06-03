#![no_main]

use libfuzzer_sys::fuzz_target;
use opentrace_bpf::utils::units::bytes::Bytes;
use opentrace_bpf::utils::units::time::{Nanoseconds, TimeAsNanosecond};

fuzz_target!(|data: &[u8]| {
    // 1. TimeAsNanosecond Display — 覆盖大跨度 u64 值的时间格式化
    if data.len() >= 8 {
        let nanos = u64::from_ne_bytes([
            data[0], data[1], data[2], data[3], data[4], data[5], data[6], data[7],
        ]);
        let ts = TimeAsNanosecond(nanos);
        let s = ts.to_string();
        // 格式化结果不应为空
        assert!(!s.is_empty());
        // 不能 panic — 除法安全（NANOS_PER_SECOND, SECONDS_PER_HOUR, etc. 都是常量 >0）
    }

    // 2. Nanoseconds Display — 覆盖所有量级 (ns, us, ms, s)
    if data.len() >= 8 {
        let nanos = u64::from_ne_bytes([
            data[0], data[1], data[2], data[3], data[4], data[5], data[6], data[7],
        ]);
        let dur = Nanoseconds(nanos);
        let s = dur.to_string();
        assert!(!s.is_empty());

        // 验证单位后缀
        assert!(s.ends_with("ns") || s.ends_with("us") || s.ends_with("ms") || s.ends_with('s'));
    }

    // 3. Bytes Display — 覆盖所有量级 (B, k, M)
    if data.len() >= 4 {
        let bytes = u32::from_ne_bytes([data[0], data[1], data[2], data[3]]);
        let b = Bytes(bytes);
        let s = b.to_string();
        assert!(!s.is_empty());

        // 验证单位后缀
        assert!(s.ends_with('B') || s.ends_with('k') || s.ends_with('M'));
    }
});
