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
    pub use super::MessageType;
    pub use super::http::{HttpDirection, HttpFrame, HttpMethod, HttpParser};
}
