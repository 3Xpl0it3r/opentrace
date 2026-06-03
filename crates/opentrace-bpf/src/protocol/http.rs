// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.
// 参考 picohttpparser 实现 HTTP/1.x 和 HTTP/2 解析
/*
字节偏移      0                   1                   2                   3
            0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
           +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
第 0-2 字节 |                         Length (24)                          |
           +---------------+---------------+-------------------------------+
第 3 字节   |     Type (8)    |
           +-------------------------------+
第 4 字节   |    Flags (8)    |
           +-------------------------------+
第 5-8 字节 |                 Stream Identifier (31)             |    R   |
           +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+    <---------------从这个地方开始就是具体的报文
第 9 字节  |                                                               |
  至       |                         Frame Payload                         |
第(9+L-1)  |                           (可变长度)                          |
  字节     |                                                               |
           +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+

*/

// 下面代码由Ai生成

use std::collections::HashMap;

use super::{MessageType, ParsedFrame};

type HeaderMap = Option<HashMap<Box<str>, Box<str>>>;

const HTTP2_PREFACE: &[u8; 24] = b"PRI * HTTP/2.0\r\n\r\nSM\r\n\r\n";
const HTTP2_FRAME_HEADER_LEN: usize = 9;
const HTTP2_FRAME_DATA: u8 = 0;
const HTTP2_FRAME_HEADERS: u8 = 1;
const HTTP2_FRAME_PRIORITY: u8 = 2;
const HTTP2_FRAME_RST_STREAM: u8 = 3;
const HTTP2_FRAME_SETTINGS: u8 = 4;
const HTTP2_FRAME_PUSH_PROMISE: u8 = 5;
const HTTP2_FRAME_PING: u8 = 6;
const HTTP2_FRAME_GOAWAY: u8 = 7;
const HTTP2_FRAME_WINDOW_UPDATE: u8 = 8;
const HTTP2_FRAME_CONTINUATION: u8 = 9;
const HTTP2_MAX_FRAME_TYPE: u8 = HTTP2_FRAME_CONTINUATION;
const HTTP2_FLAG_PADDED: u8 = 0x8;
const HTTP2_FLAG_PRIORITY: u8 = 0x20;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HttpVersion {
    Http10,
    Http11,
    Http2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Delete,
    Patch,
    Head,
    Options,
    Connect,
    Trace,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HttpDirection {
    ClientRequest,
    ServerResponse,
    Unknown,
}

impl HttpMethod {
    pub fn as_bytes(self) -> &'static [u8] {
        match self {
            Self::Get => b"GET",
            Self::Post => b"POST",
            Self::Put => b"PUT",
            Self::Delete => b"DELETE",
            Self::Patch => b"PATCH",
            Self::Head => b"HEAD",
            Self::Options => b"OPTIONS",
            Self::Connect => b"CONNECT",
            Self::Trace => b"TRACE",
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Get => "GET",
            Self::Post => "POST",
            Self::Put => "PUT",
            Self::Delete => "DELETE",
            Self::Patch => "PATCH",
            Self::Head => "HEAD",
            Self::Options => "OPTIONS",
            Self::Connect => "CONNECT",
            Self::Trace => "TRACE",
        }
    }
}

impl std::fmt::Display for HttpMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[derive(Debug, Clone)]
pub struct HttpFrame {
    pub message_type: MessageType,
    pub version: HttpVersion,
    pub stream_id: u32,
    pub method: Option<HttpMethod>,
    pub url: Option<Box<str>>,
    pub status: Option<u16>,
    pub headers: Option<HashMap<Box<str>, Box<str>>>,
    pub body: Option<Box<str>>,
}

impl ParsedFrame for HttpFrame {
    fn message_type(&self) -> MessageType {
        self.message_type
    }

    fn payload(&mut self) -> Option<Box<str>> {
        self.body.take()
    }

    fn target(&mut self) -> Option<Box<str>> {
        match self.method {
            Some(ref method) => match self.url {
                Some(ref url) => Some(format!("{} {}", method, url).into()),
                None => Some(method.as_str().into()),
            },
            None => None,
        }
    }
}

#[derive(Debug, Default)]
pub struct HttpParser {}

impl super::ProtoParser for HttpParser {
    type Output = HttpFrame;

    // 支持http版本
    // http1.0/1.1/2 这三个版本

    // 字段填充说明说明:
    // 当设置verbose=false的时候只填充 streamid, url, method 三个字段, 就直接返回（剩余的字段直接设置位NOne)
    // 当设置verbose=true的时候，解析完streamId,method,url之后，在继续解析Header和body字段
    // size 参数表示 data 缓冲区的实际大小，但是我会确保你能解析完streamid,url,method,
    // 这三个字段，后面的能解析多少就解析多少
    //
    // 解析规则:
    // 1. 按照 HTTP 协议规范/报文格式，从前往后逐个字段解
    // 1. 解析逻辑尽可能的追求高性能，(方便字节位运算的话尽可能位运算，但是不用强求) 例如method url path, 在解析method
    // 就可以固定的几个字节直接和get/post/put 对应的十六进制位运算来比较（而不用字符串比较）
    fn parse(&self, data: &[u8], size: usize, verbose: bool) -> Option<Self::Output> {
        let data = trim_to_size(data, size);

        parse_http1(data, verbose).or_else(|| parse_http2(data, verbose))
    }

    fn hash_id(&self, data: &[u8], size: usize) -> u32 {
        let data = trim_to_size(data, size);

        if let Some(frame) = parse_http2(data, false) {
            return frame.stream_id;
        }

        if let Some(frame) = parse_http1(data, false) {
            return http1_hash_id(frame.method, frame.url.as_deref(), frame.status);
        }

        0
    }
}

fn trim_to_size(data: &[u8], size: usize) -> &[u8] {
    &data[..data.len().min(size)]
}

fn parse_http1(data: &[u8], verbose: bool) -> Option<HttpFrame> {
    let line_end = find_crlf(data)?;
    let line = &data[..line_end];

    if line.starts_with(b"HTTP/1.") {
        parse_http1_response(data, line, line_end, verbose)
    } else {
        parse_http1_request(data, line, line_end, verbose)
    }
}

fn parse_http1_request<'a>(
    data: &'a [u8],
    line: &'a [u8],
    line_end: usize,
    verbose: bool,
) -> Option<HttpFrame> {
    let method_end = find_byte(line, b' ')?;
    let method = parse_method(&line[..method_end])?;
    let url_start = method_end + 1;
    let url_end = find_byte(&line[url_start..], b' ')? + url_start;
    let version = parse_http1_version(&line[url_end + 1..])?;
    let url = std::str::from_utf8(&line[url_start..url_end]).ok()?;

    if !verbose {
        return Some(HttpFrame {
            message_type: MessageType::Request,
            version,
            stream_id: 0,
            method: Some(method),
            url: Some(url.into()),
            status: None,
            headers: None,
            body: None,
        });
    }

    let (headers, body) = parse_http1_headers_and_body(data, line_end + 2);
    Some(HttpFrame {
        message_type: MessageType::Request,
        version,
        stream_id: 0,
        method: Some(method),
        url: Some(url.into()),
        status: None,
        headers,
        body,
    })
}

fn parse_http1_response<'a>(
    data: &'a [u8],
    line: &'a [u8],
    line_end: usize,
    verbose: bool,
) -> Option<HttpFrame> {
    let version = parse_http1_version(line.get(..8)?)?;
    let status = parse_status(line.get(9..12)?)?;

    if !verbose {
        return Some(HttpFrame {
            message_type: MessageType::Response,
            version,
            stream_id: 0,
            method: None,
            url: None,
            status: Some(status),
            headers: None,
            body: None,
        });
    }

    let (headers, body) = parse_http1_headers_and_body(data, line_end + 2);
    Some(HttpFrame {
        message_type: MessageType::Response,
        version,
        stream_id: 0,
        method: None,
        url: None,
        status: Some(status),
        headers,
        body,
    })
}

fn parse_http1_headers_and_body(data: &[u8], start: usize) -> (HeaderMap, Option<Box<str>>) {
    let mut headers = HashMap::new();
    let mut offset = start;

    while offset < data.len() {
        let line_end = match find_crlf(&data[offset..]) {
            Some(line_end) => offset + line_end,
            None => return (empty_map_to_none(headers), None),
        };

        if line_end == offset {
            let body_start = line_end + 2;
            let body = data.get(body_start..).and_then(body_to_boxed_str);
            return (empty_map_to_none(headers), body);
        }

        if let Some(colon) = find_byte(&data[offset..line_end], b':') {
            let name = trim_ascii(&data[offset..offset + colon]);
            let value = trim_ascii(&data[offset + colon + 1..line_end]);
            if let (Ok(name), Ok(value)) = (std::str::from_utf8(name), std::str::from_utf8(value)) {
                headers.insert(name.into(), value.into());
            }
        }

        offset = line_end + 2;
    }

    (empty_map_to_none(headers), None)
}

fn empty_map_to_none<K, V>(headers: HashMap<K, V>) -> Option<HashMap<K, V>> {
    (!headers.is_empty()).then_some(headers)
}

fn parse_http2(data: &[u8], verbose: bool) -> Option<HttpFrame> {
    let has_preface = data.starts_with(HTTP2_PREFACE.as_slice());
    let data = if has_preface {
        &data[HTTP2_PREFACE.len()..]
    } else {
        data
    };
    if data.len() < HTTP2_FRAME_HEADER_LEN {
        return None;
    }

    let frame_len = ((data[0] as usize) << 16) | ((data[1] as usize) << 8) | data[2] as usize;
    let frame_type = data[3];
    let flags = data[4];
    let stream_id = u32::from_be_bytes([data[5] & 0x7f, data[6], data[7], data[8]]);

    if !is_plausible_http2_frame(
        frame_type,
        flags,
        stream_id,
        frame_len,
        data.len(),
        has_preface,
    ) {
        return None;
    }

    let payload_end = HTTP2_FRAME_HEADER_LEN + frame_len.min(data.len() - HTTP2_FRAME_HEADER_LEN);
    let payload = &data[HTTP2_FRAME_HEADER_LEN..payload_end];

    let mut frame = HttpFrame {
        message_type: MessageType::Unknown,
        version: HttpVersion::Http2,
        stream_id,
        method: None,
        url: None,
        status: None,
        headers: None,
        body: None,
    };

    if !verbose && frame_type != HTTP2_FRAME_HEADERS {
        return Some(frame);
    }

    match frame_type {
        HTTP2_FRAME_DATA => {
            frame.body = http2_data_payload(payload, flags);
        }
        HTTP2_FRAME_HEADERS | HTTP2_FRAME_CONTINUATION => {
            parse_http2_headers(payload, flags, frame_type, verbose, &mut frame);
        }
        _ => {}
    }

    Some(frame)
}

fn parse_http2_headers(
    payload: &[u8],
    flags: u8,
    frame_type: u8,
    verbose: bool,
    frame: &mut HttpFrame,
) {
    let header_block = match http2_header_block(payload, flags, frame_type) {
        Some(b) => b,
        None => return,
    };
    let headers = parse_hpack_literals(header_block);
    frame.method = headers
        .get(":method")
        .and_then(|method| parse_method(method.as_bytes()));
    frame.url = headers.get(":path").cloned();
    frame.status = headers
        .get(":status")
        .and_then(|status| parse_status(status.as_bytes()));
    frame.message_type = match (frame.method, frame.status) {
        (Some(_), _) => MessageType::Request,
        (_, Some(_)) => MessageType::Response,
        _ => MessageType::Unknown,
    };
    if verbose {
        frame.headers = empty_map_to_none(headers);
    }
}

fn is_plausible_http2_frame(
    frame_type: u8,
    flags: u8,
    stream_id: u32,
    frame_len: usize,
    available_len: usize,
    has_preface: bool,
) -> bool {
    if frame_type > HTTP2_MAX_FRAME_TYPE || available_len < HTTP2_FRAME_HEADER_LEN {
        return false;
    }

    let available_payload_len = available_len - HTTP2_FRAME_HEADER_LEN;

    match frame_type {
        HTTP2_FRAME_DATA => stream_id != 0,
        HTTP2_FRAME_HEADERS => {
            validate_h2_headers(stream_id, has_preface, frame_len, available_payload_len)
        }
        HTTP2_FRAME_PRIORITY => validate_h2_priority(stream_id, frame_len, available_payload_len),
        HTTP2_FRAME_RST_STREAM => {
            validate_h2_rst_stream(stream_id, frame_len, available_payload_len)
        }
        HTTP2_FRAME_SETTINGS => {
            validate_h2_settings(flags, stream_id, frame_len, available_payload_len)
        }
        HTTP2_FRAME_PUSH_PROMISE => {
            validate_h2_push_promise(stream_id, has_preface, frame_len, available_payload_len)
        }
        HTTP2_FRAME_PING => validate_h2_ping(stream_id, frame_len, available_payload_len),
        HTTP2_FRAME_GOAWAY => validate_h2_goaway(stream_id, frame_len, available_payload_len),
        HTTP2_FRAME_WINDOW_UPDATE => validate_h2_window_update(frame_len, available_payload_len),
        HTTP2_FRAME_CONTINUATION => {
            validate_h2_continuation(stream_id, has_preface, frame_len, available_payload_len)
        }
        _ => false,
    }
}

fn fixed_payload_available(
    frame_len: usize,
    available_payload_len: usize,
    expected: usize,
) -> bool {
    frame_len == expected && available_payload_len >= expected
}

fn validate_h2_headers(
    stream_id: u32,
    has_preface: bool,
    frame_len: usize,
    available_payload_len: usize,
) -> bool {
    stream_id != 0 && (!has_preface || frame_len <= available_payload_len)
}

fn validate_h2_priority(stream_id: u32, frame_len: usize, available_payload_len: usize) -> bool {
    stream_id != 0 && fixed_payload_available(frame_len, available_payload_len, 5)
}

fn validate_h2_rst_stream(stream_id: u32, frame_len: usize, available_payload_len: usize) -> bool {
    stream_id != 0 && fixed_payload_available(frame_len, available_payload_len, 4)
}

fn validate_h2_settings(
    flags: u8,
    stream_id: u32,
    frame_len: usize,
    available_payload_len: usize,
) -> bool {
    stream_id == 0
        && frame_len.is_multiple_of(6)
        && (flags & 0x1 == 0 || frame_len == 0)
        && frame_len <= available_payload_len
}

fn validate_h2_push_promise(
    stream_id: u32,
    has_preface: bool,
    frame_len: usize,
    available_payload_len: usize,
) -> bool {
    stream_id != 0 && frame_len >= 4 && (!has_preface || frame_len <= available_payload_len)
}

fn validate_h2_ping(stream_id: u32, frame_len: usize, available_payload_len: usize) -> bool {
    stream_id == 0 && fixed_payload_available(frame_len, available_payload_len, 8)
}

fn validate_h2_goaway(stream_id: u32, frame_len: usize, available_payload_len: usize) -> bool {
    stream_id == 0 && frame_len >= 8 && available_payload_len >= frame_len.min(8)
}

fn validate_h2_window_update(frame_len: usize, available_payload_len: usize) -> bool {
    fixed_payload_available(frame_len, available_payload_len, 4)
}

fn validate_h2_continuation(
    stream_id: u32,
    has_preface: bool,
    frame_len: usize,
    available_payload_len: usize,
) -> bool {
    stream_id != 0 && (!has_preface || frame_len <= available_payload_len)
}

fn http2_data_payload(payload: &[u8], flags: u8) -> Option<Box<str>> {
    let payload = if flags & HTTP2_FLAG_PADDED != 0 {
        let pad_len = *payload.first()? as usize;
        payload.get(1..payload.len().checked_sub(pad_len)?)?
    } else {
        payload
    };

    body_to_boxed_str(payload)
}

fn body_to_boxed_str(body: &[u8]) -> Option<Box<str>> {
    if body.is_empty() {
        return None;
    }

    std::str::from_utf8(body).ok().map(Into::into)
}

fn http2_header_block(payload: &[u8], flags: u8, frame_type: u8) -> Option<&[u8]> {
    let mut offset = 0usize;
    let mut end = payload.len();

    if flags & HTTP2_FLAG_PADDED != 0 {
        let pad_len = *payload.first()? as usize;
        offset = 1;
        end = payload.len().checked_sub(pad_len)?;
    }

    if frame_type == HTTP2_FRAME_HEADERS && flags & HTTP2_FLAG_PRIORITY != 0 {
        offset += 5;
    }

    (offset <= end).then_some(&payload[offset..end])
}

fn parse_hpack_literals(mut data: &[u8]) -> HashMap<Box<str>, Box<str>> {
    let mut headers = HashMap::new();

    while let Some((&first, _)) = data.split_first() {
        if first & 0x80 != 0 {
            data = match try_indexed_hpack(data) {
                Some((name, value, rest)) => {
                    headers.insert(name.into(), value.into());
                    rest
                }
                None => break,
            };
            continue;
        }

        if first & 0xe0 == 0x20 {
            data = match decode_prefixed_int(data, 0x1f) {
                Some((_, rest)) => rest,
                None => break,
            };
            continue;
        }

        data = match try_literal_hpack_entry(data) {
            Some((name, value, rest)) => {
                headers.insert(name.into(), value.into());
                rest
            }
            None => break,
        };
    }

    headers
}

fn try_indexed_hpack(data: &[u8]) -> Option<(&'static str, &'static str, &[u8])> {
    let (index, rest) = decode_prefixed_int(data, 0x7f)?;
    let (name, value) = indexed_header(index)?;
    Some((name, value, rest))
}

fn try_literal_hpack_entry(data: &[u8]) -> Option<(&str, &str, &[u8])> {
    let first = *data.first()?;

    let name_index_mask = if first & 0x40 != 0 {
        0x3f
    } else if (first & 0xf0 == 0) || (first & 0xf0 == 0x10) {
        0x0f
    } else {
        return None;
    };

    let (name_index, after_name_index) = decode_prefixed_int(data, name_index_mask)?;

    let (name, after_name) = if name_index == 0 {
        decode_hpack_string(after_name_index)?
    } else {
        let (name, _) = indexed_header(name_index)?;
        (name, after_name_index)
    };

    let (value, after_value) = decode_hpack_string(after_name)?;
    Some((name, value, after_value))
}

fn decode_prefixed_int(data: &[u8], prefix_mask: u8) -> Option<(usize, &[u8])> {
    let (&first, mut rest) = data.split_first()?;
    let mut value = (first & prefix_mask) as usize;
    if value < prefix_mask as usize {
        return Some((value, rest));
    }

    let mut shift = 0usize;
    loop {
        let (&byte, next) = rest.split_first()?;
        rest = next;
        value = value.checked_add(((byte & 0x7f) as usize) << shift)?;
        if byte & 0x80 == 0 {
            return Some((value, rest));
        }
        shift += 7;
        if shift >= usize::BITS as usize {
            return None;
        }
    }
}

fn decode_hpack_string(data: &[u8]) -> Option<(&str, &[u8])> {
    let (len, rest) = decode_prefixed_int(data, 0x7f)?;
    let value = rest.get(..len)?;
    let rest = rest.get(len..)?;

    if data.first()? & 0x80 != 0 {
        return None;
    }

    Some((std::str::from_utf8(value).ok()?, rest))
}

fn indexed_header(index: usize) -> Option<(&'static str, &'static str)> {
    static TABLE: &[(&str, &str)] = &[
        ("", ""),
        ("", ""),
        (":method", "GET"),
        (":method", "POST"),
        (":path", "/"),
        (":path", "/index.html"),
        (":scheme", "http"),
        (":scheme", "https"),
        (":status", "200"),
        (":status", "204"),
        (":status", "206"),
        (":status", "304"),
        (":status", "400"),
        (":status", "404"),
        (":status", "500"),
    ];
    TABLE.get(index).copied()
}

fn parse_http1_version(data: &[u8]) -> Option<HttpVersion> {
    match data {
        b"HTTP/1.0" => Some(HttpVersion::Http10),
        b"HTTP/1.1" => Some(HttpVersion::Http11),
        _ => None,
    }
}

fn parse_method(data: &[u8]) -> Option<HttpMethod> {
    Some(match data {
        b"GET" => HttpMethod::Get,
        b"PUT" => HttpMethod::Put,
        b"POST" => HttpMethod::Post,
        b"HEAD" => HttpMethod::Head,
        b"PATCH" => HttpMethod::Patch,
        b"TRACE" => HttpMethod::Trace,
        b"DELETE" => HttpMethod::Delete,
        b"OPTIONS" => HttpMethod::Options,
        b"CONNECT" => HttpMethod::Connect,
        _ => return None,
    })
}

fn parse_status(data: &[u8]) -> Option<u16> {
    if data.len() != 3 || !data.iter().all(u8::is_ascii_digit) {
        return None;
    }

    Some(((data[0] - b'0') as u16) * 100 + ((data[1] - b'0') as u16) * 10 + (data[2] - b'0') as u16)
}

fn http1_hash_id(method: Option<HttpMethod>, url: Option<&str>, status: Option<u16>) -> u32 {
    if let (Some(method), Some(url)) = (method, url) {
        return fnv1a32(method.as_bytes()) ^ fnv1a32(url.as_bytes()).rotate_left(13);
    }

    status.map(u32::from).unwrap_or_default()
}

fn fnv1a32(data: &[u8]) -> u32 {
    let mut hash = 0x811c_9dc5u32;
    for byte in data {
        hash ^= *byte as u32;
        hash = hash.wrapping_mul(0x0100_0193);
    }
    hash
}

fn find_crlf(data: &[u8]) -> Option<usize> {
    data.windows(2).position(|window| window == b"\r\n")
}

fn find_byte(data: &[u8], byte: u8) -> Option<usize> {
    data.iter().position(|candidate| *candidate == byte)
}

fn trim_ascii(mut data: &[u8]) -> &[u8] {
    while data.first().is_some_and(u8::is_ascii_whitespace) {
        data = &data[1..];
    }
    while data.last().is_some_and(u8::is_ascii_whitespace) {
        data = &data[..data.len() - 1];
    }
    data
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::{MessageType, ParsedFrame, ProtoParser};
    use rstest::rstest;

    // ==================== parse_method 测试 ====================

    #[rstest]
    #[case(b"GET", Some(HttpMethod::Get))]
    #[case(b"POST", Some(HttpMethod::Post))]
    #[case(b"PUT", Some(HttpMethod::Put))]
    #[case(b"DELETE", Some(HttpMethod::Delete))]
    #[case(b"PATCH", Some(HttpMethod::Patch))]
    #[case(b"HEAD", Some(HttpMethod::Head))]
    #[case(b"OPTIONS", Some(HttpMethod::Options))]
    #[case(b"CONNECT", Some(HttpMethod::Connect))]
    #[case(b"TRACE", Some(HttpMethod::Trace))]
    #[case(b"INVALID", None)]
    #[case(b"get", None)]
    #[case(b"", None)]
    fn parse_method_handles_all_variants(
        #[case] input: &[u8],
        #[case] expected: Option<HttpMethod>,
    ) {
        assert_eq!(parse_method(input), expected);
    }

    // ==================== parse_status 测试 ====================

    #[rstest]
    #[case(b"200", Some(200))]
    #[case(b"404", Some(404))]
    #[case(b"500", Some(500))]
    #[case(b"000", Some(0))]
    #[case(b"999", Some(999))]
    #[case(b"20", None)] // 长度不足
    #[case(b"2000", None)] // 长度过长
    #[case(b"abc", None)] // 非数字
    #[case(b"20a", None)] // 混合字符
    fn parse_status_validates_and_converts(#[case] input: &[u8], #[case] expected: Option<u16>) {
        assert_eq!(parse_status(input), expected);
    }

    // ==================== find_crlf 测试 ====================

    #[test]
    fn find_crlf_at_start() {
        assert_eq!(find_crlf(b"\r\nrest"), Some(0));
    }

    #[test]
    fn find_crlf_in_middle() {
        assert_eq!(find_crlf(b"line1\r\nline2"), Some(5));
    }

    #[test]
    fn find_crlf_at_end() {
        assert_eq!(find_crlf(b"line\r\n"), Some(4));
    }

    #[test]
    fn find_crlf_not_found() {
        assert_eq!(find_crlf(b"no crlf here"), None);
    }

    #[test]
    fn find_crlf_empty() {
        assert_eq!(find_crlf(b""), None);
    }

    #[test]
    fn find_crlf_single_char() {
        assert_eq!(find_crlf(b"\r"), None);
    }

    // ==================== find_byte 测试 ====================

    #[test]
    fn find_byte_at_start() {
        assert_eq!(find_byte(b"abc", b'a'), Some(0));
    }

    #[test]
    fn find_byte_in_middle() {
        assert_eq!(find_byte(b"abc", b'b'), Some(1));
    }

    #[test]
    fn find_byte_at_end() {
        assert_eq!(find_byte(b"abc", b'c'), Some(2));
    }

    #[test]
    fn find_byte_not_found() {
        assert_eq!(find_byte(b"abc", b'd'), None);
    }

    #[test]
    fn find_byte_empty() {
        assert_eq!(find_byte(b"", b'a'), None);
    }

    // ==================== trim_ascii 测试 ====================

    #[test]
    fn trim_ascii_leading_whitespace() {
        assert_eq!(trim_ascii(b"  hello"), b"hello");
    }

    #[test]
    fn trim_ascii_trailing_whitespace() {
        assert_eq!(trim_ascii(b"hello  "), b"hello");
    }

    #[test]
    fn trim_ascii_both_sides() {
        assert_eq!(trim_ascii(b"  hello  "), b"hello");
    }

    #[test]
    fn trim_ascii_no_whitespace() {
        assert_eq!(trim_ascii(b"hello"), b"hello");
    }

    #[test]
    fn trim_ascii_only_whitespace() {
        assert_eq!(trim_ascii(b"   "), b"");
    }

    #[test]
    fn trim_ascii_empty() {
        assert_eq!(trim_ascii(b""), b"");
    }

    #[test]
    fn trim_ascii_tabs_and_newlines() {
        assert_eq!(trim_ascii(b"\t\nhello\r\n"), b"hello");
    }

    // ==================== fnv1a32 测试 ====================

    #[test]
    fn fnv1a32_empty() {
        // FNV-1a 初始值
        assert_eq!(fnv1a32(b""), 0x811c_9dc5);
    }

    #[test]
    fn fnv1a32_known_values() {
        // 已知的 FNV-1a 哈希值
        assert_eq!(fnv1a32(b"a"), 0xe40c292c);
        assert_eq!(fnv1a32(b"foobar"), 0xbf9cf968);
    }

    #[test]
    fn fnv1a32_deterministic() {
        assert_eq!(fnv1a32(b"test"), fnv1a32(b"test"));
    }

    #[test]
    fn fnv1a32_different_inputs_differ() {
        assert_ne!(fnv1a32(b"a"), fnv1a32(b"b"));
    }

    // ==================== body_to_boxed_str 测试 ====================

    #[test]
    fn body_to_boxed_str_valid_utf8() {
        assert_eq!(body_to_boxed_str(b"hello").as_deref(), Some("hello"));
    }

    #[test]
    fn body_to_boxed_str_empty() {
        assert_eq!(body_to_boxed_str(b""), None);
    }

    #[test]
    fn body_to_boxed_str_invalid_utf8() {
        assert_eq!(body_to_boxed_str(&[0xff, 0xfe]), None);
    }

    // ==================== decode_prefixed_int 测试 ====================

    #[rstest]
    #[case(&[0x00], 0x0f, Some((0, [].as_slice())))]
    #[case(&[0x05], 0x0f, Some((5, [].as_slice())))]
    #[case(&[0x0f, 0x00], 0x0f, Some((15, [].as_slice())))]
    #[case(&[0x0f, 0x09], 0x0f, Some((24, [].as_slice())))]
    #[case(&[0x0f, 0x80, 0x01], 0x0f, Some((143, [].as_slice())))]
    fn decode_prefixed_int_basic(
        #[case] data: &[u8],
        #[case] mask: u8,
        #[case] expected: Option<(usize, &[u8])>,
    ) {
        assert_eq!(decode_prefixed_int(data, mask), expected);
    }

    #[test]
    fn decode_prefixed_int_empty() {
        assert_eq!(decode_prefixed_int(&[], 0x0f), None);
    }

    // ==================== indexed_header 测试 ====================

    #[rstest]
    #[case(0, Some(("", "")))]
    #[case(2, Some((":method", "GET")))]
    #[case(3, Some((":method", "POST")))]
    #[case(4, Some((":path", "/")))]
    #[case(8, Some((":status", "200")))]
    #[case(14, Some((":status", "500")))]
    #[case(15, None)] // 超出范围
    #[case(100, None)] // 超出范围
    fn indexed_header_lookups(#[case] index: usize, #[case] expected: Option<(&str, &str)>) {
        assert_eq!(indexed_header(index), expected);
    }

    // ==================== http1_hash_id 测试 ====================

    #[test]
    fn http1_hash_id_with_method_and_url() {
        let hash = http1_hash_id(Some(HttpMethod::Get), Some("/hello"), None);
        assert_ne!(hash, 0);
    }

    #[test]
    fn http1_hash_id_deterministic() {
        let h1 = http1_hash_id(Some(HttpMethod::Get), Some("/hello"), None);
        let h2 = http1_hash_id(Some(HttpMethod::Get), Some("/hello"), None);
        assert_eq!(h1, h2);
    }

    #[test]
    fn http1_hash_id_different_urls_differ() {
        let h1 = http1_hash_id(Some(HttpMethod::Get), Some("/a"), None);
        let h2 = http1_hash_id(Some(HttpMethod::Get), Some("/b"), None);
        assert_ne!(h1, h2);
    }

    #[test]
    fn http1_hash_id_fallback_to_status() {
        let hash = http1_hash_id(None, None, Some(200));
        assert_eq!(hash, 200);
    }

    #[test]
    fn http1_hash_id_returns_zero_when_no_info() {
        assert_eq!(http1_hash_id(None, None, None), 0);
    }

    // ==================== validate_h2_* 函数测试 ====================

    #[test]
    fn validate_h2_settings_valid() {
        // stream_id == 0, frame_len 是 6 的倍数
        assert!(validate_h2_settings(0, 0, 6, 6));
        assert!(validate_h2_settings(0, 0, 12, 12));
    }

    #[test]
    fn validate_h2_settings_non_zero_stream() {
        // stream_id 必须为 0
        assert!(!validate_h2_settings(0, 1, 6, 6));
    }

    #[test]
    fn validate_h2_settings_not_multiple_of_6() {
        assert!(!validate_h2_settings(0, 0, 5, 5));
    }

    #[test]
    fn validate_h2_settings_ack_flag_requires_empty() {
        // ACK flag (0x1) 时 frame_len 必须为 0
        assert!(validate_h2_settings(0x1, 0, 0, 0));
        assert!(!validate_h2_settings(0x1, 0, 6, 6));
    }

    #[test]
    fn validate_h2_ping_valid() {
        assert!(validate_h2_ping(0, 8, 8));
    }

    #[test]
    fn validate_h2_ping_non_zero_stream() {
        assert!(!validate_h2_ping(1, 8, 8));
    }

    #[test]
    fn validate_h2_ping_wrong_length() {
        assert!(!validate_h2_ping(0, 4, 4));
    }

    #[test]
    fn validate_h2_rst_stream_valid() {
        assert!(validate_h2_rst_stream(1, 4, 4));
    }

    #[test]
    fn validate_h2_rst_stream_zero_stream() {
        assert!(!validate_h2_rst_stream(0, 4, 4));
    }

    #[test]
    fn validate_h2_rst_stream_wrong_length() {
        assert!(!validate_h2_rst_stream(1, 8, 8));
    }

    #[test]
    fn validate_h2_priority_valid() {
        assert!(validate_h2_priority(1, 5, 5));
    }

    #[test]
    fn validate_h2_priority_wrong_length() {
        assert!(!validate_h2_priority(1, 4, 4));
    }

    #[test]
    fn validate_h2_window_update_valid() {
        assert!(validate_h2_window_update(4, 4));
    }

    #[test]
    fn validate_h2_window_update_wrong_length() {
        assert!(!validate_h2_window_update(8, 8));
    }

    #[test]
    fn validate_h2_goaway_valid() {
        assert!(validate_h2_goaway(0, 8, 8));
        assert!(validate_h2_goaway(0, 16, 16));
    }

    #[test]
    fn validate_h2_goaway_non_zero_stream() {
        assert!(!validate_h2_goaway(1, 8, 8));
    }

    #[test]
    fn validate_h2_goaway_too_short() {
        assert!(!validate_h2_goaway(0, 4, 4));
    }

    // ==================== http2_header_block 测试 ====================

    #[test]
    fn http2_header_block_no_padding() {
        let payload = &[0x82, 0x84];
        assert_eq!(
            http2_header_block(payload, 0, HTTP2_FRAME_HEADERS),
            Some(payload.as_slice())
        );
    }

    #[test]
    fn http2_header_block_with_padding() {
        // padded flag, pad_len = 2
        let payload = &[0x02, 0x82, 0x84, 0x00, 0x00];
        let result = http2_header_block(payload, HTTP2_FLAG_PADDED, HTTP2_FRAME_HEADERS);
        assert_eq!(result, Some(&[0x82, 0x84][..]));
    }

    #[test]
    fn http2_header_block_with_priority() {
        // priority flag, 5 bytes priority data
        let payload = &[0x00, 0x00, 0x00, 0x00, 0x00, 0x82, 0x84];
        let result = http2_header_block(payload, HTTP2_FLAG_PRIORITY, HTTP2_FRAME_HEADERS);
        assert_eq!(result, Some(&[0x82, 0x84][..]));
    }

    // ==================== http2_data_payload 测试 ====================

    #[test]
    fn http2_data_payload_no_padding() {
        let payload = b"hello";
        assert_eq!(http2_data_payload(payload, 0).as_deref(), Some("hello"));
    }

    #[test]
    fn http2_data_payload_with_padding() {
        // pad_len = 2, data = "hi", padding = [0, 0]
        let payload = &[0x02, b'h', b'i', 0x00, 0x00];
        assert_eq!(
            http2_data_payload(payload, HTTP2_FLAG_PADDED).as_deref(),
            Some("hi")
        );
    }

    #[test]
    fn http2_data_payload_empty() {
        assert_eq!(http2_data_payload(b"", 0), None);
    }

    // ==================== empty_map_to_none 测试 ====================

    #[test]
    fn empty_map_to_none_empty() {
        let map: HashMap<Box<str>, Box<str>> = HashMap::new();
        assert!(empty_map_to_none(map).is_none());
    }

    #[test]
    fn empty_map_to_none_non_empty() {
        let mut map = HashMap::<String, String>::new();
        map.insert("key".into(), "value".into());
        assert!(empty_map_to_none(map).is_some());
    }

    // ==================== trim_to_size 测试 ====================

    #[test]
    fn trim_to_size_smaller_than_data() {
        assert_eq!(trim_to_size(b"hello", 3), b"hel");
    }

    #[test]
    fn trim_to_size_larger_than_data() {
        assert_eq!(trim_to_size(b"hello", 10), b"hello");
    }

    #[test]
    fn trim_to_size_equal() {
        assert_eq!(trim_to_size(b"hello", 5), b"hello");
    }

    // ==================== 原有测试 ====================

    #[test]
    fn parses_http1_request_minimal_fields() {
        let parser = HttpParser::default();
        let frame = parser
            .parse(
                b"GET /hello HTTP/1.1\r\nHost: example\r\n\r\nbody",
                64,
                false,
            )
            .unwrap();

        assert_eq!(frame.message_type, MessageType::Request);
        assert_eq!(frame.version, HttpVersion::Http11);
        assert_eq!(frame.method, Some(HttpMethod::Get));
        assert_eq!(frame.url.as_deref(), Some("/hello"));
        assert!(frame.headers.is_none());
        assert!(frame.body.is_none());
    }

    #[test]
    fn parses_http1_request_headers_and_body_when_verbose() {
        let parser = HttpParser::default();
        let frame = parser
            .parse(
                b"POST /submit HTTP/1.1\r\nHost: example\r\n\r\npayload",
                128,
                true,
            )
            .unwrap();

        assert_eq!(frame.method, Some(HttpMethod::Post));
        assert_eq!(
            frame.headers.as_ref().unwrap().get("Host").map(Box::as_ref),
            Some("example")
        );
        assert_eq!(frame.body.as_deref(), Some("payload"));
    }

    #[test]
    fn parses_http1_response_status() {
        let parser = HttpParser::default();
        let frame = parser.parse(b"HTTP/1.1 200 OK\r\n\r\n", 32, false).unwrap();

        assert_eq!(frame.message_type, MessageType::Response);
        assert_eq!(frame.status, Some(200));
    }

    #[test]
    fn target_consumes_method_and_url() {
        let mut frame = HttpFrame {
            message_type: MessageType::Request,
            version: HttpVersion::Http11,
            stream_id: 0,
            method: Some(HttpMethod::Get),
            url: Some("/x".into()),
            status: None,
            headers: None,
            body: Some("body".into()),
        };

        assert_eq!(frame.target().as_deref(), Some("GET /x"));
        assert_eq!(frame.payload().as_deref(), Some("body"));
        assert!(frame.payload().is_none());
    }

    #[test]
    fn parses_http2_preface_settings_frame() {
        let mut data = Vec::from(HTTP2_PREFACE.as_slice());
        data.extend_from_slice(&[0, 0, 0, HTTP2_FRAME_SETTINGS, 0, 0, 0, 0, 0]);

        let parser = HttpParser::default();
        let frame = parser.parse(&data, data.len(), false).unwrap();

        assert_eq!(frame.version, HttpVersion::Http2);
        assert_eq!(frame.stream_id, 0);
        assert_eq!(frame.message_type, MessageType::Unknown);
    }

    #[test]
    fn parses_simple_http2_indexed_headers() {
        let data = [0, 0, 2, HTTP2_FRAME_HEADERS, 0, 0, 0, 0, 1, 0x82, 0x84];

        let parser = HttpParser::default();
        let frame = parser.parse(&data, data.len(), false).unwrap();

        assert_eq!(frame.message_type, MessageType::Request);
        assert_eq!(frame.method, Some(HttpMethod::Get));
        assert_eq!(frame.url.as_deref(), Some("/"));
    }

    #[test]
    fn hash_id_is_stable_for_same_http1_target() {
        let parser = HttpParser::default();
        let left = parser.hash_id(b"GET /hello HTTP/1.1\r\n\r\n", 24);
        let right = parser.hash_id(b"GET /hello HTTP/1.1\r\nHost: x\r\n\r\n", 128);

        assert_ne!(left, 0);
        assert_eq!(left, right);
    }

    #[test]
    fn rejects_invalid_http_payload() {
        let parser = HttpParser::default();
        assert!(parser.parse(b"not http", 8, false).is_none());
        assert_eq!(parser.hash_id(b"not http", 8), 0);
    }
}
