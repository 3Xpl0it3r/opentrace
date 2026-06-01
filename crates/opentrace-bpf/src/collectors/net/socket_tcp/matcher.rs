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
