#![no_main]

use libfuzzer_sys::fuzz_target;
use opentrace_bpf::utils::cstring::from_bytes_lossy;

fuzz_target!(|data: &[u8]| {
    // from_bytes_lossy 从任意字节序列中提取 C 字符串
    // 使用 CStr::from_bytes_until_nul，可能返回空字符串或 lossy UTF-8
    let s = from_bytes_lossy(data);

    // 不能 panic，不能包含 nul 字符
    assert!(!s.contains('\0'));

    // 结果应该总是有效的 String（不会再 panic）
    let _ = s.len();
    let _ = s.is_empty();
});
