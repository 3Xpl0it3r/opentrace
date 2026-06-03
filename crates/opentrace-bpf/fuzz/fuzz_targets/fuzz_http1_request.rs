#![no_main]

use libfuzzer_sys::fuzz_target;
use opentrace_bpf::protocol::ProtoParser;
use opentrace_bpf::protocol::appproto::HttpParser;

/// 构造一个看起来像 HTTP/1.x 请求的数据
fn make_http1_request_like(data: &[u8]) -> Vec<u8> {
    // 尝试将数据解释为 HTTP/1.x 请求格式
    // 格式: METHOD /path HTTP/1.1\r\nHeaders\r\n\r\nBody
    let mut request = Vec::new();

    // 添加一个有效的 HTTP 方法前缀
    request.extend_from_slice(b"GET /");

    // 添加用户数据作为路径和头部
    request.extend_from_slice(data);

    // 确保有 CRLF 结束
    if !data.windows(2).any(|w| w == b"\r\n") {
        request.extend_from_slice(b" HTTP/1.1\r\n\r\n");
    }

    request
}

fuzz_target!(|data: &[u8]| {
    let parser = HttpParser::default();

    // 1. 直接解析原始数据
    let _ = parser.parse(data, data.len(), false);
    let _ = parser.parse(data, data.len(), true);

    // 2. 解析构造的 HTTP/1.x 请求
    let request = make_http1_request_like(data);
    let _ = parser.parse(&request, request.len(), false);
    let _ = parser.parse(&request, request.len(), true);

    // 3. 测试不同 size 参数
    for size in [0, 1, data.len() / 2, data.len(), data.len() * 2] {
        let _ = parser.parse(data, size, false);
    }

    // 4. 测试 hash_id
    let _ = parser.hash_id(data, data.len());
});
