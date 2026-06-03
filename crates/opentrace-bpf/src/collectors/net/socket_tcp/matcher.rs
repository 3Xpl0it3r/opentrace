use std::collections::HashMap;

use crate::protocol::{ParsedFrame, ProtoParser};
use crate::types::net::Addr;

use super::event::{Event, InnerEvent};

// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.
enum ConnectionKind {
    Unknown = 0,
    Active = 1,
    Positive = 2,
}
impl From<u32> for ConnectionKind {
    fn from(value: u32) -> Self {
        match value {
            1 => ConnectionKind::Active,
            2 => ConnectionKind::Positive,
            _ => ConnectionKind::Unknown,
        }
    }
}

enum FlowDirection {
    Unknown = 0,
    Ingress = 1,
    Egress = 2,
}
impl From<u32> for FlowDirection {
    fn from(value: u32) -> Self {
        match value {
            1 => FlowDirection::Ingress,
            2 => FlowDirection::Egress,
            _ => FlowDirection::Unknown,
        }
    }
}

#[derive(Default)]
pub(super) struct EventMatcher<T> {
    // pid fd Addr
    passive_conns: HashMap<(u32 /*pid*/, u32 /*fd*/), HashMap<Addr, Event>>,
    active_conns: HashMap<(u32, u32), HashMap<Addr, Event>>,
    proto_parser: T,
    verbose: bool,
}

impl<T> EventMatcher<T>
where
    T: ProtoParser<Output: ParsedFrame>,
{
    pub(super) fn new(proto_parser: T, verbose: bool) -> Self {
        Self {
            passive_conns: HashMap::new(),
            active_conns: HashMap::new(),
            proto_parser,
            verbose,
        }
    }

    fn handle_request_active(
        conns: &mut HashMap<(u32, u32), HashMap<Addr, Event>>,
        mut frame: impl ParsedFrame,
        conn_key: (u32, u32),
        addr: Addr,
        i_event: InnerEvent,
    ) {
        let mut event: Event = i_event.into();
        event.req_body = frame.payload();
        event.target = frame.target();
        conns.entry(conn_key).or_default().insert(addr, event);
    }

    fn handle_response_active(
        conns: &mut HashMap<(u32, u32), HashMap<Addr, Event>>,
        mut frame: impl ParsedFrame,
        conn_key: (u32, u32),
        addr: Addr,
        i_event: InnerEvent,
        _verbose: bool,
    ) -> Option<Event> {
        let map = conns.get_mut(&conn_key)?;
        let mut event = map.remove(&addr)?;
        if map.is_empty() {
            conns.remove(&conn_key);
        }
        event.response_size = i_event.size;
        event.duration = i_event.timestamp - event.timestamp;
        event.timestamp = i_event.timestamp;
        event.resp_body = frame.payload();
        Some(event)
    }

    // 1. 如果类型是event类型是active主动发起的，则存入active_conns，并从active_conns里面寻找配对
    // 2. 如果event 类型是positive的，则存入positive_conns，并从positive_conns里面寻找配对
    pub(super) fn try_match(&mut self, event: InnerEvent) -> Option<Event> {
        let conn_key = (event.pid, event.fd);
        let addr = event.remote_addr;

        let frame = self
            .proto_parser
            .parse(&event.buffer, event.size as usize, self.verbose)?;

        match (
            ConnectionKind::from(event.conn_kind),
            FlowDirection::from(event.flow_direct),
        ) {
            (ConnectionKind::Active, FlowDirection::Egress) => {
                Self::handle_request_active(&mut self.active_conns, frame, conn_key, addr, event);
                None
            }
            (ConnectionKind::Active, FlowDirection::Ingress) => Self::handle_response_active(
                &mut self.active_conns,
                frame,
                conn_key,
                addr,
                event,
                self.verbose,
            ),
            (ConnectionKind::Positive, FlowDirection::Ingress) => {
                Self::handle_request_active(&mut self.passive_conns, frame, conn_key, addr, event);
                None
            }
            (ConnectionKind::Positive, FlowDirection::Egress) => Self::handle_response_active(
                &mut self.passive_conns,
                frame,
                conn_key,
                addr,
                event,
                self.verbose,
            ),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{EventMatcher, InnerEvent};
    use crate::protocol::{MessageType, ParsedFrame, ProtoParser};
    use crate::types::net::Addr;

    #[derive(Default)]
    struct TestParser;

    struct TestFrame {
        payload: Option<Box<str>>,
        target: Option<Box<str>>,
    }

    impl ParsedFrame for TestFrame {
        fn message_type(&self) -> MessageType {
            MessageType::Request
        }

        fn payload(&mut self) -> Option<Box<str>> {
            self.payload.take()
        }

        fn target(&mut self) -> Option<Box<str>> {
            self.target.take()
        }
    }

    impl ProtoParser for TestParser {
        type Output = TestFrame;

        fn parse(&self, data: &[u8], size: usize, _verbose: bool) -> Option<Self::Output> {
            let payload = std::str::from_utf8(&data[..data.len().min(size)]).ok()?;
            Some(TestFrame {
                payload: Some(payload.into()),
                target: Some("GET /".into()),
            })
        }

        fn hash_id(&self, _data: &[u8], _size: usize) -> u32 {
            0
        }
    }

    fn inner_event(conn_kind: u32, flow_direct: u32, timestamp: u64, body: &str) -> InnerEvent {
        let mut buffer = [0; 1024];
        buffer[..body.len()].copy_from_slice(body.as_bytes());
        InnerEvent {
            buffer,
            remote_addr: Addr {
                v4addr: u32::from_ne_bytes([10, 0, 0, 1]),
            },
            local_addr: Addr {
                v4addr: u32::from_ne_bytes([10, 0, 0, 2]),
            },
            timestamp,
            size: body.len() as u32,
            pid: 1,
            fd: 2,
            conn_kind,
            flow_direct,
            remote_port: 8080,
            local_port: 80,
            family: 2,
            _pad: 0,
        }
    }

    #[test]
    fn active_connection_matches_egress_request_to_ingress_response() {
        let mut matcher = EventMatcher::new(TestParser, false);

        assert!(
            matcher
                .try_match(inner_event(1, 2, 10, "request"))
                .is_none()
        );
        let event = matcher
            .try_match(inner_event(1, 1, 25, "response"))
            .unwrap();

        assert_eq!(event.req_body.as_deref(), Some("request"));
        assert_eq!(event.resp_body.as_deref(), Some("response"));
        assert_eq!(event.duration, 15);
        assert_eq!(event.response_size, 8);
        assert_eq!(event.target.as_deref(), Some("GET /"));
    }

    #[test]
    fn passive_connection_matches_ingress_request_to_egress_response() {
        let mut matcher = EventMatcher::new(TestParser, false);

        assert!(
            matcher
                .try_match(inner_event(2, 1, 10, "request"))
                .is_none()
        );
        let event = matcher
            .try_match(inner_event(2, 2, 20, "response"))
            .unwrap();

        assert_eq!(event.req_body.as_deref(), Some("request"));
        assert_eq!(event.resp_body.as_deref(), Some("response"));
    }

    #[test]
    fn unmatched_response_returns_none() {
        let mut matcher = EventMatcher::new(TestParser, false);

        assert!(
            matcher
                .try_match(inner_event(1, 1, 20, "response"))
                .is_none()
        );
    }
}
