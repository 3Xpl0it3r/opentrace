#![no_main]

use libfuzzer_sys::fuzz_target;
use opentrace_bpf::protocol::ProtoParser;
use opentrace_bpf::protocol::appproto::HttpParser;

/// 构造一个 HPACK 编码的 HEADERS 帧
fn make_hpack_headers_frame(data: &[u8]) -> Vec<u8> {
    let mut frame = Vec::new();

    // 帧头: Length (3 bytes) + Type (1 byte) + Flags (1 byte) + Stream ID (4 bytes)
    let len = data.len();
    frame.push(((len >> 16) & 0xFF) as u8);
    frame.push(((len >> 8) & 0xFF) as u8);
    frame.push((len & 0xFF) as u8);
    frame.push(0x01); // HEADERS frame type
    frame.push(0x00); // No flags
    frame.extend_from_slice(&[0x00, 0x00, 0x00, 0x01]); // Stream ID = 1

    // HPACK payload
    frame.extend_from_slice(data);

    frame
}

/// 构造一个带 PADDED 标志的 HEADERS 帧
fn make_padded_headers_frame(data: &[u8]) -> Vec<u8> {
    let mut frame = Vec::new();

    let payload_len = data.len() + 1; // +1 for pad length byte
    frame.push(((payload_len >> 16) & 0xFF) as u8);
    frame.push(((payload_len >> 8) & 0xFF) as u8);
    frame.push((payload_len & 0xFF) as u8);
    frame.push(0x01); // HEADERS frame type
    frame.push(0x08); // PADDED flag
    frame.extend_from_slice(&[0x00, 0x00, 0x00, 0x01]); // Stream ID = 1

    frame.push(0x00); // Pad length = 0
    frame.extend_from_slice(data);

    frame
}

/// 构造一个带 PRIORITY 标志的 HEADERS 帧
fn make_priority_headers_frame(data: &[u8]) -> Vec<u8> {
    let mut frame = Vec::new();

    let payload_len = data.len() + 5; // +5 for priority data
    frame.push(((payload_len >> 16) & 0xFF) as u8);
    frame.push(((payload_len >> 8) & 0xFF) as u8);
    frame.push((payload_len & 0xFF) as u8);
    frame.push(0x01); // HEADERS frame type
    frame.push(0x20); // PRIORITY flag
    frame.extend_from_slice(&[0x00, 0x00, 0x00, 0x01]); // Stream ID = 1

    // Priority data (5 bytes)
    frame.extend_from_slice(&[0x00, 0x00, 0x00, 0x00, 0x00]);
    frame.extend_from_slice(data);

    frame
}

fuzz_target!(|data: &[u8]| {
    let parser = HttpParser::default();

    // 1. 直接用原始数据构造 HEADERS 帧
    let headers_frame = make_hpack_headers_frame(data);
    let _ = parser.parse(&headers_frame, headers_frame.len(), false);
    let _ = parser.parse(&headers_frame, headers_frame.len(), true);

    // 2. 带 PADDED 的 HEADERS 帧
    let padded_frame = make_padded_headers_frame(data);
    let _ = parser.parse(&padded_frame, padded_frame.len(), false);
    let _ = parser.parse(&padded_frame, padded_frame.len(), true);

    // 3. 带 PRIORITY 的 HEADERS 帧
    let priority_frame = make_priority_headers_frame(data);
    let _ = parser.parse(&priority_frame, priority_frame.len(), false);
    let _ = parser.parse(&priority_frame, priority_frame.len(), true);

    // 4. 带 HTTP/2 preface 的 HEADERS 帧
    let mut with_preface = Vec::from(b"PRI * HTTP/2.0\r\n\r\nSM\r\n\r\n".as_slice());
    with_preface.extend_from_slice(&headers_frame);
    let _ = parser.parse(&with_preface, with_preface.len(), false);
    let _ = parser.parse(&with_preface, with_preface.len(), true);
});
