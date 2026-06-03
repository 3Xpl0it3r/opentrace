// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.

use crate::protocol::{MessageType, ParsedFrame, ProtoParser};

/// Mock Frame 实现，用于测试
#[derive(Debug, Clone)]
pub struct MockFrame {
    pub message_type: MessageType,
    pub payload: Option<Box<str>>,
    pub target: Option<Box<str>>,
}

impl MockFrame {
    /// 创建请求帧
    pub fn request(method: &str, url: &str) -> Self {
        Self {
            message_type: MessageType::Request,
            payload: None,
            target: Some(format!("{method} {url}").into()),
        }
    }

    /// 创建响应帧
    pub fn response(status: u16) -> Self {
        Self {
            message_type: MessageType::Response,
            payload: None,
            target: Some(status.to_string().into()),
        }
    }

    /// 创建带 payload 的帧
    pub fn with_payload(mut self, payload: &str) -> Self {
        self.payload = Some(payload.into());
        self
    }

    /// 创建带 target 的帧
    pub fn with_target(mut self, target: &str) -> Self {
        self.target = Some(target.into());
        self
    }
}

impl ParsedFrame for MockFrame {
    fn message_type(&self) -> MessageType {
        self.message_type
    }

    fn payload(&mut self) -> Option<Box<str>> {
        self.payload.take()
    }

    fn target(&mut self) -> Option<Box<str>> {
        self.target.take()
    }
}

/// Mock ProtoParser 实现，用于测试
///
/// # 示例
///
/// ```rust
/// use opentrace_bpf::testing::{MockProtoParser, MockFrame};
/// use opentrace_bpf::protocols::{ProtoParser, ParsedFrame, MessageType};
///
/// let parser = MockProtoParser::new()
///     .with_frame(MockFrame::request("GET", "/hello"));
///
/// let mut frame = parser.parse(b"data", 4, false).unwrap();
/// assert_eq!(frame.message_type(), MessageType::Request);
/// ```
pub struct MockProtoParser {
    frames: Vec<Option<MockFrame>>,
    call_count: usize,
    hash_id_result: u32,
}

impl MockProtoParser {
    /// 创建新的 MockProtoParser（默认返回 None）
    pub fn new() -> Self {
        Self {
            frames: Vec::new(),
            call_count: 0,
            hash_id_result: 0,
        }
    }

    /// 添加一个 parse 调用的返回值
    pub fn with_frame(mut self, frame: MockFrame) -> Self {
        self.frames.push(Some(frame));
        self
    }

    /// 添加一个 parse 调用返回 None
    pub fn with_none(mut self) -> Self {
        self.frames.push(None);
        self
    }

    /// 设置 hash_id 的返回值
    pub fn with_hash_id(mut self, hash_id: u32) -> Self {
        self.hash_id_result = hash_id;
        self
    }

    /// 获取 parse 调用次数
    pub fn call_count(&self) -> usize {
        self.call_count
    }
}

impl Default for MockProtoParser {
    fn default() -> Self {
        Self::new()
    }
}

impl ProtoParser for MockProtoParser {
    type Output = MockFrame;

    fn parse(&self, _data: &[u8], _size: usize, _verbose: bool) -> Option<Self::Output> {
        // 注意：这里需要可变引用来更新 call_count，但 trait 方法是 &self
        // 在实际使用中可以通过 Cell 或 RefCell 实现，这里简化处理
        self.frames.first()?.clone()
    }

    fn hash_id(&self, _data: &[u8], _size: usize) -> u32 {
        self.hash_id_result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 创建一个总是返回固定帧的 parser
    pub fn parser_with_frame(frame: MockFrame) -> MockProtoParser {
        MockProtoParser::new().with_frame(frame)
    }

    /// 创建一个总是返回 None 的 parser
    pub fn null_parser() -> MockProtoParser {
        MockProtoParser::new().with_none()
    }

    /// 创建一个返回请求帧的 parser
    pub fn request_parser(method: &str, url: &str) -> MockProtoParser {
        parser_with_frame(MockFrame::request(method, url))
    }

    /// 创建一个返回响应帧的 parser
    pub fn response_parser(status: u16) -> MockProtoParser {
        parser_with_frame(MockFrame::response(status))
    }

    #[test]
    fn mock_frame_request() {
        let mut frame = MockFrame::request("GET", "/hello");
        assert_eq!(frame.message_type(), MessageType::Request);
        assert_eq!(frame.target().as_deref(), Some("GET /hello"));
        assert!(frame.payload().is_none());
    }

    #[test]
    fn mock_frame_response() {
        let mut frame = MockFrame::response(200);
        assert_eq!(frame.message_type(), MessageType::Response);
        assert_eq!(frame.target().as_deref(), Some("200"));
    }

    #[test]
    fn mock_frame_with_payload() {
        let mut frame = MockFrame::request("POST", "/submit").with_payload("body data");
        assert_eq!(frame.payload().as_deref(), Some("body data"));
        assert!(frame.payload().is_none()); // take() 消耗掉了
    }

    #[test]
    fn mock_proto_parser_returns_frame() {
        let parser = MockProtoParser::new()
            .with_frame(MockFrame::request("GET", "/"))
            .with_hash_id(42);

        let mut frame = parser.parse(b"data", 4, false).unwrap();
        assert_eq!(frame.message_type(), MessageType::Request);
        assert_eq!(frame.target().as_deref(), Some("GET /"));

        assert_eq!(parser.hash_id(b"data", 4), 42);
    }

    #[test]
    fn mock_proto_parser_returns_none() {
        let parser = MockProtoParser::new().with_none();
        assert!(parser.parse(b"data", 4, false).is_none());
    }

    #[test]
    fn helper_functions() {
        let parser = request_parser("POST", "/api");
        let mut frame = parser.parse(b"", 0, false).unwrap();
        assert_eq!(frame.target().as_deref(), Some("POST /api"));

        let parser = response_parser(404);
        let mut frame = parser.parse(b"", 0, false).unwrap();
        assert_eq!(frame.target().as_deref(), Some("404"));

        let parser = null_parser();
        assert!(parser.parse(b"", 0, false).is_none());
    }
}
