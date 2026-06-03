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

#[cfg(test)]
mod tests {
    use super::{Event, InnerEvent};
    use crate::types::net::Addr;

    fn inner_event() -> InnerEvent {
        InnerEvent {
            buffer: [0; 1024],
            remote_addr: Addr {
                v4addr: u32::from_ne_bytes([10, 0, 0, 1]),
            },
            local_addr: Addr {
                v4addr: u32::from_ne_bytes([10, 0, 0, 2]),
            },
            timestamp: 123,
            size: 456,
            pid: 1,
            fd: 2,
            conn_kind: 1,
            flow_direct: 2,
            remote_port: 8080,
            local_port: 80,
            family: 2,
            _pad: 0,
        }
    }

    #[test]
    fn converts_inner_event_to_public_request_event() {
        let event = Event::from(inner_event());

        assert_eq!(event.remote_port, 8080);
        assert_eq!(event.family, 2);
        assert_eq!(event.timestamp, 123);
        assert_eq!(event.request_size, 456);
        assert_eq!(event.response_size, 0);
        assert!(event.req_body.is_none());
        assert!(event.resp_body.is_none());
    }
}
