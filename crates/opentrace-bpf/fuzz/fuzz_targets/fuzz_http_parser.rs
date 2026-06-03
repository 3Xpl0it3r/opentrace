#![no_main]

use libfuzzer_sys::fuzz_target;
use opentrace_bpf::protocols::ProtoParser;
use opentrace_bpf::protocols::app_protos::HttpParser;

fuzz_target!(|data: &[u8]| {
    let parser = HttpParser::default();

    // Fuzz HTTP 解析（verbose=false）
    let _ = parser.parse(data, data.len(), false);

    // Fuzz HTTP 解析（verbose=true）
    let _ = parser.parse(data, data.len(), true);

    // Fuzz hash_id 计算
    let _ = parser.hash_id(data, data.len());
});
