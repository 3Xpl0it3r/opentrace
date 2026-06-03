#![no_main]

use libfuzzer_sys::fuzz_target;
use opentrace_bpf::protocol::appproto::HttpParser;
use opentrace_bpf::protocol::{ParsedFrame, ProtoParser};

fuzz_target!(|data: &[u8]| {
    let parser = HttpParser::default();

    // 1. 测试不同 size 参数
    for size in [0, 1, data.len() / 2, data.len(), data.len() * 2] {
        let _ = parser.parse(data, size, false);
        let _ = parser.parse(data, size, true);
    }

    // 2. 测试 HTTP/1.x 请求格式
    let request_prefix = b"GET / HTTP/1.1\r\n";
    let mut request = Vec::from(request_prefix.as_slice());
    request.extend_from_slice(data);
    request.extend_from_slice(b"\r\n\r\n");

    if let Some(mut frame) = parser.parse(&request, request.len(), false) {
        let _ = frame.message_type();
        let _ = frame.payload();
        let _ = frame.target();
    }

    // 3. 测试 HTTP/1.x 响应格式
    let response_prefix = b"HTTP/1.1 200 OK\r\n";
    let mut response = Vec::from(response_prefix.as_slice());
    response.extend_from_slice(data);
    response.extend_from_slice(b"\r\n\r\n");

    if let Some(mut frame) = parser.parse(&response, response.len(), false) {
        let _ = frame.message_type();
        let _ = frame.payload();
        let _ = frame.target();
    }

    // 4. 测试 HTTP/2 preface
    let http2_preface = b"PRI * HTTP/2.0\r\n\r\nSM\r\n\r\n";
    let mut http2_data = Vec::from(http2_preface.as_slice());
    http2_data.extend_from_slice(data);

    if let Some(mut frame) = parser.parse(&http2_data, http2_data.len(), false) {
        let _ = frame.message_type();
        let _ = frame.payload();
        let _ = frame.target();
    }

    // 5. 测试 hash_id
    let _ = parser.hash_id(data, data.len());
    let _ = parser.hash_id(&request, request.len());
    let _ = parser.hash_id(&response, response.len());
    let _ = parser.hash_id(&http2_data, http2_data.len());
});
