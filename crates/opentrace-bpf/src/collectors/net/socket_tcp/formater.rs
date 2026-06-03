// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.
// DefaultFormatter[#TODO] (shoule add some comments )

use crate::format::StreamFormatter;
use crate::protocols::ip_proto::family;
use crate::types::net::{Addr, AddrV4, AddrV6};
use crate::utils::units::{bytes, time};

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

        if event.family == family::AF_INET {
            display_simple_event_as_v4(w, event, target);
        } else {
            display_simple_event_as_v6(w, event, target);
        }
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
    if family == family::AF_INET {
        let _ = writeln!(w, "远程主机: {}:{}", AddrV4::from(addr), port);
    } else {
        let _ = writeln!(w, "远程主机: {}:{}", AddrV6::from(addr), port);
    }
}
#[inline]
fn display_simple_event_as_v4(w: &mut impl std::io::Write, event: &Event, target: &str) {
    let _ = writeln!(
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
    let _ = writeln!(
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

#[cfg(test)]
mod tests {
    use crate::format::StreamFormatter;
    use crate::types::net::Addr;

    use super::{DefaultFormatter, Event};

    fn event() -> Event {
        Event {
            remote_addr: Addr {
                v4addr: u32::from_ne_bytes([10, 0, 0, 1]),
            },
            remote_port: 8080,
            family: 2,
            req_body: Some("GET / HTTP/1.1".into()),
            resp_body: Some("HTTP/1.1 200 OK".into()),
            timestamp: 1_000_000_000,
            duration: 2_000_000,
            request_size: 100,
            response_size: 2048,
            target: Some("GET /".into()),
        }
    }

    #[test]
    fn formats_v6_simple_event_line() {
        let mut event = event();
        event.family = 10; // AF_INET6

        let mut out = Vec::new();
        DefaultFormatter::new(false)
            .format(&mut out, &event)
            .unwrap();

        let output = String::from_utf8(out).unwrap();
        assert!(output.contains("00:00:01"));
        assert!(output.contains("GET /"));
    }

    #[test]
    fn formats_v6_remote_host_in_verbose_mode() {
        let mut event = event();
        event.family = 10; // AF_INET6

        let mut out = Vec::new();
        DefaultFormatter::new(true)
            .format(&mut out, &event)
            .unwrap();

        let output = String::from_utf8(out).unwrap();
        assert!(output.contains("远程主机:"));
        assert!(output.contains(":8080"));
    }

    #[test]
    fn verbose_mode_unknown_target() {
        let mut event = event();
        event.target = None;

        let mut out = Vec::new();
        DefaultFormatter::new(true)
            .format(&mut out, &event)
            .unwrap();

        let output = String::from_utf8(out).unwrap();
        assert!(output.contains("target:   unknown"));
    }

    #[test]
    fn simple_mode_unknown_target() {
        let mut event = event();
        event.target = None;

        let mut out = Vec::new();
        DefaultFormatter::new(false)
            .format(&mut out, &event)
            .unwrap();

        let output = String::from_utf8(out).unwrap();
        assert!(output.contains("unknown"));
    }

    #[test]
    fn verbose_mode_truncates_long_request_body() {
        let mut event = event();
        event.req_body = Some("x".repeat(200).into());

        let mut out = Vec::new();
        DefaultFormatter::new(true)
            .format(&mut out, &event)
            .unwrap();

        let output = String::from_utf8(out).unwrap();
        // 只显示前 128 字节
        assert_eq!(output.matches('x').count(), 128);
    }

    #[test]
    fn verbose_mode_truncates_long_response_body() {
        let mut event = event();
        event.resp_body = Some("y".repeat(200).into());

        let mut out = Vec::new();
        DefaultFormatter::new(true)
            .format(&mut out, &event)
            .unwrap();

        let output = String::from_utf8(out).unwrap();
        assert_eq!(output.matches('y').count(), 128);
    }

    #[test]
    fn verbose_mode_multiline_request_body() {
        let mut event = event();
        event.req_body = Some("line1\nline2\nline3".into());

        let mut out = Vec::new();
        DefaultFormatter::new(true)
            .format(&mut out, &event)
            .unwrap();

        let output = String::from_utf8(out).unwrap();
        assert!(output.contains("请求数据: line1"));
        assert!(output.contains("          line2"));
        assert!(output.contains("          line3"));
    }

    #[test]
    fn verbose_mode_no_request_or_response_body() {
        let mut event = event();
        event.req_body = None;
        event.resp_body = None;

        let mut out = Vec::new();
        DefaultFormatter::new(true)
            .format(&mut out, &event)
            .unwrap();

        let output = String::from_utf8(out).unwrap();
        assert!(output.contains("请求数据: None"));
        assert!(output.contains("响应数据: None"));
    }

    #[test]
    fn formats_simple_event_line() {
        let mut out = Vec::new();

        DefaultFormatter::new(false)
            .format(&mut out, &event())
            .unwrap();

        let output = String::from_utf8(out).unwrap();
        assert!(output.contains("00:00:01"));
        assert!(output.contains("10.0.0.1"));
        assert!(output.contains("GET /"));
        assert!(output.contains("resp_size: 2k"));
    }

    #[test]
    fn formats_verbose_event_details() {
        let mut out = Vec::new();

        DefaultFormatter::new(true)
            .format(&mut out, &event())
            .unwrap();

        let output = String::from_utf8(out).unwrap();
        assert!(output.contains("远程主机: 10.0.0.1:8080"));
        assert!(output.contains("请求数据: GET / HTTP/1.1"));
        assert!(output.contains("响应数据: HTTP/1.1 200 OK"));
        assert!(output.contains("处理时长: 2ms"));
    }
}
