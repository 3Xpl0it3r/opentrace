// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.
// DefaultFormatter[#TODO] (shoule add some comments )

use std::fmt::Write;

use crate::format::StreamFormatter;
use crate::protocols::ip_proto::family;
use crate::types::net::{Addr, AddrV4, AddrV6};
use crate::utils::{bytes, time};

use super::event::Event;

const DEFAULT_MAX_PAYLOAD_SIZE: usize = 128;

pub struct DefaultFormatter {
    verbose: bool,
}

impl DefaultFormatter {
    pub fn new(verbose: bool) -> Self {
        Self { verbose }
    }
}

impl StreamFormatter<Event> for DefaultFormatter {
    fn format<W: std::io::Write>(&self, w: &mut W, event: &Event) -> std::io::Result<()> {
        let target = event.target.as_deref().unwrap_or("unknown");

        if self.verbose {
            let _ = writeln!(w, "请求时间: {}", time::TimeAsNanosecond(event.timestamp));
            display_remote_host_information(w, event.remote_addr, event.remote_port, event.family);
            let _ = writeln!(w, "target:   {}", target);
            if let Some(ref req) = event.req_body {
                let display = if req.len() > DEFAULT_MAX_PAYLOAD_SIZE {
                    &req[..DEFAULT_MAX_PAYLOAD_SIZE]
                } else {
                    req
                };
                let lines: Vec<&str> = display.lines().collect();
                if let Some((first, rest)) = lines.split_first() {
                    let _ = writeln!(w, "请求数据: {}", first);
                    for line in rest {
                        let _ = writeln!(w, "          {}", line);
                    }
                }
            } else {
                let _ = writeln!(w, "请求数据: None");
            }
            if let Some(ref resp) = event.resp_body {
                let display = if resp.len() > DEFAULT_MAX_PAYLOAD_SIZE {
                    &resp[..DEFAULT_MAX_PAYLOAD_SIZE]
                } else {
                    resp
                };
                let lines: Vec<&str> = display.lines().collect();
                if let Some((first, rest)) = lines.split_first() {
                    let _ = writeln!(w, "响应数据: {}", first);
                    for line in rest {
                        let _ = writeln!(w, "          {}", line);
                    }
                }
            } else {
                let _ = writeln!(w, "响应数据: None");
            }
            let _ = writeln!(w, "请求大小: {}", bytes::Bytes(event.request_size));
            let _ = writeln!(w, "响应大小: {}", bytes::Bytes(event.response_size));
            let _ = writeln!(w, "处理时长: {}", time::Nanoseconds(event.duration));
            let _ = writeln!(w, "-------------------------------------------------------");
            return Ok(());
        }

        let addr_str = if event.family == family::AF_INET as u16 {
            display_simple_event_as_v4(w, event, target);
        } else {
            display_simple_event_as_v6(w, event, target);
        };
        Ok(())
    }
}

#[inline]
fn display_remote_host_information(
    w: &mut impl std::io::Write,
    addr: Addr,
    port: u16,
    family: u16,
) {
    if family == family::AF_INET as u16 {
        let _ = writeln!(w, "远程主机: {}:{}", AddrV4::from(addr), port);
    } else {
        let _ = writeln!(w, "远程主机: {}:{}", AddrV6::from(addr), port);
    }
}
#[inline]
fn display_simple_event_as_v4(w: &mut impl std::io::Write, event: &Event, target: &str) {
    writeln!(
        w,
        "{} | {} | {} | req_size: {} | resp_size: {} | cost: {}",
        time::TimeAsNanosecond(event.timestamp),
        AddrV4::from(event.remote_addr),
        target,
        bytes::Bytes(event.request_size),
        bytes::Bytes(event.response_size),
        time::Nanoseconds(event.duration)
    );
}
fn display_simple_event_as_v6(w: &mut impl std::io::Write, event: &Event, target: &str) {
    writeln!(
        w,
        "{} | {} | {} | req_size: {} | resp_size: {} | cost: {}",
        time::TimeAsNanosecond(event.timestamp),
        AddrV6::from(event.remote_addr),
        target,
        bytes::Bytes(event.request_size),
        bytes::Bytes(event.response_size),
        time::Nanoseconds(event.duration)
    );
}
