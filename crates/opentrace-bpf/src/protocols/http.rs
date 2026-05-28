// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.
// AI 生成 xiaomiv2.5pro
/*
字节偏移      0                   1                   2                   3
            0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
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
//

const HTTP2_PREFIX: &[u8] = b"PRI * HTTP/2.0\r\n\r\nSM\r\n\r\n";
const FRAME_HEADER_LEN: usize = 9;
// HTTP/2 frame type 合法范围 0x00..=0x09 (DATA..CONTINUATION)
const MAX_FRAME_TYPE: u8 = 0x09;
const FRAME_TYPE_HEADERS: u8 = 0x01;
// HEADERS flags
const FLAG_PADDED: u8 = 0x08;
const FLAG_PRIORITY: u8 = 0x20;
// HPACK 静态表里 :path 的索引
const HPACK_STATIC_PATH_INDEX: u8 = 4;
// HPACK 静态表里 :method 相关索引
const HPACK_STATIC_METHOD_NAME: u8 = 2; // :method (作为 name index 用)
const HPACK_STATIC_METHOD_GET: u8 = 2; // :method GET (作为 indexed header 用)
const HPACK_STATIC_METHOD_POST: u8 = 3; // :method POST (作为 indexed header 用)
// 未协商前 SETTINGS_MAX_FRAME_SIZE 默认 16384
const DEFAULT_MAX_FRAME_SIZE: usize = 16_384;

// HTTP/1.x 请求方法前缀
const HTTP1_METHODS: &[(&[u8], RequestMethod)] = &[
    (b"GET ", RequestMethod::GET),
    (b"POST ", RequestMethod::POST),
    (b"PUT ", RequestMethod::Unknown),
    (b"HEAD ", RequestMethod::Unknown),
    (b"DELETE ", RequestMethod::Unknown),
    (b"PATCH ", RequestMethod::Unknown),
    (b"OPTIONS ", RequestMethod::Unknown),
];

#[derive(Debug, Clone, Copy)]
pub enum HttpVersion {
    Unknown,
    HTTP1_0,
    HTTP1_1,
    HTTP2_0,
}

impl Default for HttpVersion {
    fn default() -> Self {
        HttpVersion::Unknown
    }
}

#[derive(Debug, Clone, Copy)]
pub enum RequestMethod {
    Unknown,
    GET,
    POST,
}

impl Default for RequestMethod {
    fn default() -> Self {
        RequestMethod::Unknown
    }
}

#[derive(Debug, Default)]
pub struct Frame {
    pub version: HttpVersion,
    pub method: RequestMethod,
    pub url: String,
    pub stream_id: u32, // http2里面的stream id
    pub frame_type: u8,
    pub payload: Vec<u8>,
}

#[derive(Default)]
pub struct Parser;

impl super::ProtoParser for Parser {
    type Output<'a> = Frame;
    fn parse<'a>(&self, raw_data: &'a [u8], size: usize) -> Self::Output<'a> {
        Self::parse_inner(raw_data, size).unwrap_or_default()
    }
}

impl Parser {
    fn parse_inner(raw: &[u8], datasize: usize) -> Option<Frame> {
        let raw = &raw[..datasize.min(raw.len())];
        // 尝试解析 HTTP/1.x
        if let Some(frame) = parse_http1(raw) {
            return Some(frame);
        }

        // 跳过 HTTP/2 连接前导（如果存在）
        let mut buf = raw;
        if buf.starts_with(HTTP2_PREFIX) {
            buf = &buf[HTTP2_PREFIX.len()..];
        }

        if buf.len() < FRAME_HEADER_LEN {
            return None;
        }

        // 解析 9 字节帧头
        let length = ((buf[0] as usize) << 16) | ((buf[1] as usize) << 8) | (buf[2] as usize);
        let frame_type = buf[3];
        let flags = buf[4];
        let stream_id = (((buf[5] & 0x7f) as u32) << 24)
            | ((buf[6] as u32) << 16)
            | ((buf[7] as u32) << 8)
            | (buf[8] as u32);

        if frame_type > MAX_FRAME_TYPE {
            return None;
        }
        if length > DEFAULT_MAX_FRAME_SIZE {
            return None;
        }

        // 取 payload，截断到实际可用长度，避免越界
        let payload_end = FRAME_HEADER_LEN.saturating_add(length).min(buf.len());
        let body = &buf[FRAME_HEADER_LEN..payload_end];

        let mut frame = Frame {
            version: HttpVersion::HTTP2_0,
            method: RequestMethod::Unknown,
            url: String::new(),
            stream_id,
            frame_type,
            payload: body.to_vec(),
        };

        if frame_type == FRAME_TYPE_HEADERS {
            if let Some(block) = strip_headers_padding_and_priority(body, flags) {
                let (path, method) = decode_hpack_minimal(block);
                if let Some(p) = path {
                    frame.url = p;
                }
                if let Some(m) = method {
                    frame.method = m;
                }
            }
        }

        Some(frame)
    }
}

// 解析 HTTP/1.x 请求和响应
fn parse_http1(raw: &[u8]) -> Option<Frame> {
    // HTTP/1.x 请求: "METHOD /path HTTP/1.x\r\n..."
    for &(prefix, method) in HTTP1_METHODS {
        if raw.starts_with(prefix) {
            let rest = &raw[prefix.len()..];
            // 找到第一个空格，提取 path
            if let Some(space_pos) = rest.iter().position(|&b| b == b' ') {
                let path = std::str::from_utf8(&rest[..space_pos]).ok()?;
                return Some(Frame {
                    version: HttpVersion::HTTP1_1,
                    method,
                    url: path.to_string(),
                    stream_id: 0,
                    frame_type: 0,
                    payload: Vec::new(),
                });
            }
        }
    }

    // HTTP/1.x 响应: "HTTP/1.x STATUS ..."
    if raw.starts_with(b"HTTP/1.0 ") || raw.starts_with(b"HTTP/1.1 ") {
        return Some(Frame {
            version: if raw.starts_with(b"HTTP/1.0 ") {
                HttpVersion::HTTP1_0
            } else {
                HttpVersion::HTTP1_1
            },
            method: RequestMethod::Unknown,
            url: String::new(),
            stream_id: 0,
            frame_type: 0,
            payload: Vec::new(),
        });
    }

    None
}

// 处理 HEADERS 帧的 PADDED / PRIORITY 标志，返回真正的 HPACK 区块
fn strip_headers_padding_and_priority<'a>(mut body: &'a [u8], flags: u8) -> Option<&'a [u8]> {
    if flags & FLAG_PADDED != 0 {
        if body.is_empty() {
            return None;
        }
        let pad_len = body[0] as usize;
        body = &body[1..];
        if pad_len > body.len() {
            return None;
        }
        body = &body[..body.len() - pad_len];
    }
    if flags & FLAG_PRIORITY != 0 {
        if body.len() < 5 {
            return None;
        }
        body = &body[5..];
    }
    Some(body)
}

// 解 HPACK 整数：第 1 字节低 prefix_bits 位作为初值，若达到上限则用后续字节扩展
fn decode_hpack_integer(data: &[u8], prefix_bits: u8) -> Option<(u64, &[u8])> {
    if data.is_empty() || prefix_bits == 0 || prefix_bits > 8 {
        return None;
    }
    let mask: u8 = ((1u16 << prefix_bits) - 1) as u8;
    let mut value = (data[0] & mask) as u64;
    if value < mask as u64 {
        return Some((value, &data[1..]));
    }
    let mut i = 1usize;
    let mut shift: u32 = 0;
    while i < data.len() {
        let b = data[i];
        i += 1;
        value = value.checked_add(((b & 0x7f) as u64).checked_shl(shift)?)?;
        if b & 0x80 == 0 {
            return Some((value, &data[i..]));
        }
        shift = shift.checked_add(7)?;
        if shift > 63 {
            return None;
        }
    }
    None
}

// 读取一个 HPACK 字符串：返回 (huffman_flag, bytes, rest)。
// Huffman 编码暂不解码，调用方需自行判断。
fn read_hpack_string(data: &[u8]) -> Option<(bool, &[u8], &[u8])> {
    if data.is_empty() {
        return None;
    }
    let huffman = data[0] & 0x80 != 0;
    let (len, rest) = decode_hpack_integer(data, 7)?;
    let len = len as usize;
    if len > rest.len() {
        return None;
    }
    Some((huffman, &rest[..len], &rest[len..]))
}

// 极简 HPACK 解码器：只提取 :path 与 :method（不处理 Huffman 编码的字符串）
fn decode_hpack_minimal(mut data: &[u8]) -> (Option<String>, Option<RequestMethod>) {
    let mut path: Option<String> = None;
    let mut method: Option<RequestMethod> = None;

    while !data.is_empty() {
        let b = data[0];

        if b & 0x80 != 0 {
            // Indexed Header Field
            let (idx, rest) = match decode_hpack_integer(data, 7) {
                Some(x) => x,
                None => break,
            };
            data = rest;
            match idx as u8 {
                HPACK_STATIC_METHOD_GET => {
                    method.get_or_insert(RequestMethod::GET);
                }
                HPACK_STATIC_METHOD_POST => {
                    method.get_or_insert(RequestMethod::POST);
                }
                4 => {
                    path.get_or_insert_with(|| "/".to_string());
                }
                5 => {
                    path.get_or_insert_with(|| "/index.html".to_string());
                }
                _ => {}
            }
        } else if b & 0x40 != 0 {
            // Literal Header Field with Incremental Indexing (prefix=6)
            let (name_idx, rest) = match decode_hpack_integer(data, 6) {
                Some(x) => x,
                None => break,
            };
            data = rest;
            if name_idx == 0 {
                // 名字也是字面量，跳过
                let (_huff, _name, rest) = match read_hpack_string(data) {
                    Some(x) => x,
                    None => break,
                };
                data = rest;
            }
            let (huff, value, rest) = match read_hpack_string(data) {
                Some(x) => x,
                None => break,
            };
            data = rest;
            apply_literal(name_idx as u8, huff, value, &mut path, &mut method);
        } else if b & 0x20 != 0 {
            // Dynamic Table Size Update (prefix=5)，忽略具体大小
            let (_, rest) = match decode_hpack_integer(data, 5) {
                Some(x) => x,
                None => break,
            };
            data = rest;
        } else {
            // Literal Header Field without Indexing / Never Indexed (prefix=4)
            let (name_idx, rest) = match decode_hpack_integer(data, 4) {
                Some(x) => x,
                None => break,
            };
            data = rest;
            if name_idx == 0 {
                let (_huff, _name, rest) = match read_hpack_string(data) {
                    Some(x) => x,
                    None => break,
                };
                data = rest;
            }
            let (huff, value, rest) = match read_hpack_string(data) {
                Some(x) => x,
                None => break,
            };
            data = rest;
            apply_literal(name_idx as u8, huff, value, &mut path, &mut method);
        }
    }

    (path, method)
}

fn apply_literal(
    name_idx: u8,
    huffman: bool,
    value: &[u8],
    path: &mut Option<String>,
    method: &mut Option<RequestMethod>,
) {
    // Huffman 编码的字符串这里不解码，直接放弃
    if huffman {
        return;
    }
    if name_idx == HPACK_STATIC_PATH_INDEX {
        if let Ok(s) = std::str::from_utf8(value) {
            *path = Some(s.to_string());
        }
    } else if name_idx == HPACK_STATIC_METHOD_NAME {
        match value {
            b"GET" => *method = Some(RequestMethod::GET),
            b"POST" => *method = Some(RequestMethod::POST),
            _ => {}
        }
    }
}
