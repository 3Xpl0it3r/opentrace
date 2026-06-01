// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.
// DefaultFormatter[#TODO] (shoule add some comments )

use crate::format::StreamFormatter;
use crate::types::net::AddrV4;

use super::event::Event;

const DEFAULT_MAX_PAYLOAD_SIZE: usize = 128;

fn format_duration(duration: u64) -> String {
    if duration >= 1_000_000_000 {
        format!("{}s", duration / 1_000_000_000)
    } else if duration >= 1_000_000 {
        format!("{}ms", duration / 1_000_000)
    } else if duration >= 1_000 {
        format!("{}us", duration / 1_000)
    } else {
        format!("{}ns", duration)
    }
}

fn format_size(size: u32) -> String {
    if size >= 1024 * 1024 {
        format!("{}M", size / (1024 * 1024))
    } else if size >= 1024 {
        format!("{}k", size / 1024)
    } else {
        format!("{}B", size)
    }
}
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
        let duration_str = format_duration(event.duration);
        let target = event.target.as_deref().unwrap_or("unknown");

        if self.verbose {
            let _ = writeln!(
                w,
                "远程主机: {}:{}",
                AddrV4::from(event.remote_addr),
                event.remote_port
            );
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
            let _ = writeln!(w, "请求大小: {}", format_size(event.request_size));
            let _ = writeln!(w, "响应大小: {}", format_size(event.response_size));
            let _ = writeln!(w, "处理时长: {}", duration_str);
            let _ = writeln!(w, "-------------------------------------------------------");
        } else {
            let _ = writeln!(
                w,
                "{}:{}  cost: {}  请求数据量: {}  响应数据量: {}  {}",
                AddrV4::from(event.remote_addr),
                event.remote_port,
                duration_str,
                format_size(event.request_size),
                format_size(event.response_size),
                target
            );
        }
        Ok(())
    }
}
