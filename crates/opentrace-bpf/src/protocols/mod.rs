// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.
mod inet;
mod ether;
pub mod http;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MessageType {
    Request,
    Response,
    Unknown,
}

// 一个trait
pub trait ParsedFrame {
    fn message_type(&self) -> MessageType;
    fn payload(&mut self) -> Option<Box<str>>;
    // 操作
    fn target(&mut self) -> Option<Box<str>>; //
}

pub trait ProtoParser {
    type Output: ParsedFrame;
    fn parse(&self, data: &[u8], size: usize, verbose: bool) -> Option<Self::Output>;
    fn hash_id(&self, data: &[u8], size: usize) -> u32;
}

pub mod ip_proto {
    pub use super::inet::*;
}

pub mod eth_proto {
    pub use super::ether::*;
}

pub mod app_protos {
    pub use super::http::{HttpDirection, HttpFrame, HttpMethod, HttpParser};
}

#[cfg(test)]
mod tests {
    use super::{MessageType, ParsedFrame, ProtoParser};

    struct Frame(Option<Box<str>>);

    impl ParsedFrame for Frame {
        fn message_type(&self) -> MessageType {
            MessageType::Request
        }

        fn payload(&mut self) -> Option<Box<str>> {
            self.0.take()
        }

        fn target(&mut self) -> Option<Box<str>> {
            Some("target".into())
        }
    }

    struct Parser;

    impl ProtoParser for Parser {
        type Output = Frame;

        fn parse(&self, data: &[u8], size: usize, _verbose: bool) -> Option<Self::Output> {
            Some(Frame(Some(String::from_utf8_lossy(&data[..size]).into())))
        }

        fn hash_id(&self, _data: &[u8], size: usize) -> u32 {
            size as u32
        }
    }

    #[test]
    fn parser_and_frame_traits_are_usable() {
        let parser = Parser;
        let mut frame = parser.parse(b"abc", 3, false).unwrap();

        assert_eq!(frame.message_type(), MessageType::Request);
        assert_eq!(frame.payload().as_deref(), Some("abc"));
        assert_eq!(frame.target().as_deref(), Some("target"));
        assert_eq!(parser.hash_id(b"abc", 3), 3);
    }
}
