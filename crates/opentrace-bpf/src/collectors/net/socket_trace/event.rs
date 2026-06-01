use crate::types::net::Addr;

// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.
#[derive(Clone)]
pub struct Event {
    pub remote_addr: Addr,
    pub remote_port: u16,
    pub family: u16,
    pub req_body: Option<Box<str>>,
    pub resp_body: Option<Box<str>>,
    pub timestamp: u64,
    pub duration: u64,
    pub request_size: u32,
    pub response_size: u32,
    pub target: Option<Box<str>>,
}

// 只有在处理request的时候才会需要做转换
impl From<InnerEvent> for Event {
    fn from(value: InnerEvent) -> Self {
        Self {
            remote_addr: value.remote_addr,
            remote_port: value.remote_port,
            family: value.family,
            req_body: None,
            resp_body: None,
            timestamp: value.timestamp,
            duration: 0,
            request_size: value.size,
            response_size: 0,
            target: None,
        }
    }
}
#[derive(Clone)]
#[repr(C)]
pub(super) struct InnerEvent {
    pub(super) buffer: [u8; 1024],
    pub(super) remote_addr: Addr,
    pub(super) local_addr: Addr,
    pub(super) timestamp: u64,
    pub(super) size: u32,
    pub(super) pid: u32,
    pub(super) fd: u32,
    pub(super) conn_kind: u32,
    pub(super) flow_direct: u32,
    pub(super) remote_port: u16,
    pub(super) local_port: u16,
    pub(super) family: u16,
    pub(super) _pad: u16,
}
