#![no_main]

use libfuzzer_sys::fuzz_target;
use opentrace_bpf::protocols::ProtoParser;
use opentrace_bpf::protocols::app_protos::HttpParser;

/// 构造一个看起来像 HTTP/2 帧的数据
fn make_http2_frame_like(data: &[u8]) -> Vec<u8> {
    let mut frame = Vec::new();

    // HTTP/2 帧头格式:
    // - Length (3 bytes)
    // - Type (1 byte)
    // - Flags (1 byte)
    // - Stream Identifier (4 bytes, first bit reserved)

    if data.len() >= 9 {
        // 使用用户数据作为帧头
        frame.extend_from_slice(&data[..9]);

        // 添加一些 payload
        if data.len() > 9 {
            frame.extend_from_slice(&data[9..]);
        }
    } else {
        // 构造一个最小的帧头
        frame.extend_from_slice(&[0, 0, 0, 0, 0, 0, 0, 0, 1]); // SETTINGS frame, stream_id=1
        frame.extend_from_slice(data);
    }

    frame
}

/// 构造带 HTTP/2 preface 的数据
fn make_http2_with_preface(data: &[u8]) -> Vec<u8> {
    let mut result = Vec::new();

    // HTTP/2 preface
    result.extend_from_slice(b"PRI * HTTP/2.0\r\n\r\nSM\r\n\r\n");

    // 添加用户数据作为帧
    result.extend_from_slice(data);

    result
}

fuzz_target!(|data: &[u8]| {
    let parser = HttpParser::default();

    // 1. 直接解析原始数据
    let _ = parser.parse(data, data.len(), false);
    let _ = parser.parse(data, data.len(), true);

    // 2. 解析构造的 HTTP/2 帧
    let frame = make_http2_frame_like(data);
    let _ = parser.parse(&frame, frame.len(), false);
    let _ = parser.parse(&frame, frame.len(), true);

    // 3. 解析带 preface 的数据
    let with_preface = make_http2_with_preface(data);
    let _ = parser.parse(&with_preface, with_preface.len(), false);
    let _ = parser.parse(&with_preface, with_preface.len(), true);

    // 4. 测试 hash_id
    let _ = parser.hash_id(data, data.len());
    let _ = parser.hash_id(&frame, frame.len());
    let _ = parser.hash_id(&with_preface, with_preface.len());
});
