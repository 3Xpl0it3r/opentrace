// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.
//

use std::collections::HashMap;
use std::mem::{self, MaybeUninit};
use std::slice;

use libbpf_rs::skel::{OpenSkel, SkelBuilder};
use libbpf_rs::{MapCore, MapFlags, OpenObject, PerfBufferBuilder};

use crate::bpf::socket_trace::{SocketTraceSkel, SocketTraceSkelBuilder};
use crate::collectors::macros::{
    attach_kprobe, attach_kretprobe, attach_multiple_tracepoints, attach_tracepoint,
    define_collector,
};
use crate::exporters::{Exporter, helper::load_and_dispath_with};
use crate::format::StreamFormatter;
use crate::protocols::{ParsedFrame, ProtoParser};
use crate::skeleton::with_custom_btf_open_opts;
use crate::types::net::{Addr, AddrV4};
use crate::{EbpfError, ProbeRegistry};

const CONFIG_KEY: u8 = 0;

const DEFAULT_MAX_PAYLOAD_SIZE: usize = 128;

pub struct Config {
    pub custom_btf_path: Option<String>,
    pub pid: u32,
    pub verbose: bool,
}

#[derive(Clone)]
pub struct Event {
    pub remote_addr: Addr,
    pub remote_port: u16,
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

#[derive(Clone)]
#[repr(C)]
struct InnerEvent {
    buffer: [u8; 1024],
    remote_addr: Addr,
    local_addr: Addr,
    timestamp: u64,
    size: u32,
    pid: u32,
    fd: u32,
    conn_kind: u32,
    flow_direct: u32,
    remote_port: u16,
    local_port: u16,
}

#[repr(C)]
struct InnerConfig {
    pid: u32,
}

impl InnerConfig {
    fn as_bytes(&self) -> &[u8] {
        let ptr = self as *const Self as *const u8;
        let len = mem::size_of_val(self);
        unsafe { slice::from_raw_parts(ptr, len) }
    }
}

impl From<Config> for InnerConfig {
    fn from(value: Config) -> Self {
        InnerConfig { pid: value.pid }
    }
}

#[derive(Default)]
pub struct EventCacheStorage<T> {
    // pid fd Addr
    passive_conns: HashMap<(u32 /*pid*/, u32 /*fd*/), HashMap<Addr, Event>>,
    active_conns: HashMap<(u32, u32), HashMap<Addr, Event>>,
    proto_parser: T,
    verbose: bool,
}

impl<T> EventCacheStorage<T>
where
    T: ProtoParser<Output: ParsedFrame>,
{
    fn new(proto_parser: T, verbose: bool) -> Self {
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
    fn try_match(&mut self, event: InnerEvent) -> Option<Event> {
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

define_collector!(Collector, SocketTraceSkel);

impl<'a> Collector<'a> {
    pub fn new(
        object: &'a mut MaybeUninit<OpenObject>,
        registry: &'a ProbeRegistry,
        config: Config,
        mut exporter: impl Exporter<Event> + 'a,
        proto_parser: impl ProtoParser<Output = impl ParsedFrame> + 'a,
    ) -> Result<Self, EbpfError> {
        let skel = match config.custom_btf_path {
            Some(ref custom_btf_path) => with_custom_btf_open_opts(custom_btf_path, |open_opts| {
                Ok(SocketTraceSkelBuilder::default()
                    .open_opts(open_opts, object)?
                    .load()?)
            })?,
            None => SocketTraceSkelBuilder::default().open(object)?.load()?,
        };
        let verbose = config.verbose;
        let _ = skel.maps.config_map.update(
            &CONFIG_KEY.to_ne_bytes(),
            Into::<InnerConfig>::into(config).as_bytes(),
            MapFlags::ANY,
        );

        let mut event_cache = EventCacheStorage::new(proto_parser, verbose);
        let perf_buffer = PerfBufferBuilder::new(&skel.maps.perf_events)
            .sample_cb(move |_cpu: i32, data: &[u8]| {
                load_and_dispath_with(data, &mut exporter, |data| {
                    let inner_event =
                        unsafe { std::ptr::read_unaligned(data.as_ptr() as *const InnerEvent) };
                    event_cache.try_match(inner_event)
                });
            })
            .build()?;
        Ok(Self {
            probe_registry: registry,
            skel,
            perf_buffer,
            _links: Vec::new(),
        })
    }

    fn do_attach_probes(&mut self) -> Result<(), crate::EbpfError> {
        // tcp connect
        attach_tracepoint!(self, "syscalls", tp_sys_enter_connect);
        attach_kprobe!(self, kp_tcp_connect, "tcp_connect");
        attach_kretprobe!(self, kret_tcp_connect, "tcp_connect");
        attach_tracepoint!(self, "syscalls", tp_sys_exit_connect);

        // tcp accept
        let accept_enter_tps = ["sys_enter_accept", "sys_enter_accept4"];
        let accept_exit_tps = ["sys_exit_accept", "sys_exit_accept4"];
        attach_multiple_tracepoints!(self, "syscalls", tp_sys_enter_accept, accept_enter_tps);
        attach_kretprobe!(self, kret_sock_alloc, "sock_alloc");
        attach_multiple_tracepoints!(self, "syscalls", tp_sys_exit_accept, accept_exit_tps);

        // tcp read
        attach_tracepoint!(self, "syscalls", tp_sys_enter_read);
        attach_tracepoint!(self, "syscalls", tp_sys_enter_readv);
        attach_tracepoint!(self, "syscalls", tp_sys_enter_recv);
        attach_tracepoint!(self, "syscalls", tp_sys_enter_recvfrom);
        attach_tracepoint!(self, "syscalls", tp_sys_enter_recvmsg);
        attach_tracepoint!(self, "syscalls", tp_sys_enter_recvmmsg);
        attach_kprobe!(self, kp_security_socket_recvmsg, "security_socket_recvmsg");
        attach_tracepoint!(self, "syscalls", tp_sys_exit_read);
        attach_tracepoint!(self, "syscalls", tp_sys_exit_recv);
        attach_tracepoint!(self, "syscalls", tp_sys_exit_recvfrom);
        attach_tracepoint!(self, "syscalls", tp_sys_exit_readv);
        attach_tracepoint!(self, "syscalls", tp_sys_exit_recvmsg);
        attach_tracepoint!(self, "syscalls", tp_sys_exit_recvmmsg);

        // tcp write/send
        attach_kprobe!(self, kp_security_socket_sendmsg, "security_socket_sendmsg");
        attach_tracepoint!(self, "syscalls", tp_sys_enter_write);
        attach_tracepoint!(self, "syscalls", tp_sys_enter_writev);
        attach_tracepoint!(self, "syscalls", tp_sys_enter_send);
        attach_tracepoint!(self, "syscalls", tp_sys_enter_sendto);
        attach_tracepoint!(self, "syscalls", tp_sys_enter_sendmsg);
        attach_tracepoint!(self, "syscalls", tp_sys_enter_sendmmsg);
        attach_tracepoint!(self, "syscalls", tp_sys_exit_write);
        attach_tracepoint!(self, "syscalls", tp_sys_exit_writev);
        attach_tracepoint!(self, "syscalls", tp_sys_exit_send);
        attach_tracepoint!(self, "syscalls", tp_sys_exit_sendto);
        attach_tracepoint!(self, "syscalls", tp_sys_exit_sendmsg);
        attach_tracepoint!(self, "syscalls", tp_sys_exit_sendmmsg);

        // tcp close
        attach_tracepoint!(self, "syscalls", tp_sys_enter_close);
        attach_tracepoint!(self, "syscalls", tp_sys_exit_close);

        Ok(())
    }
}

// DefaultFormatter[#TODO] (shoule add some comments )
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
            writeln!(
                w,
                "远程主机: {}:{}",
                AddrV4::from(event.remote_addr),
                event.remote_port
            );
            writeln!("target:   {}", target);
            if let Some(ref req) = event.req_body {
                let display = if req.len() > DEFAULT_MAX_PAYLOAD_SIZE {
                    &req[..DEFAULT_MAX_PAYLOAD_SIZE]
                } else {
                    req
                };
                let lines: Vec<&str> = display.lines().collect();
                if let Some((first, rest)) = lines.split_first() {
                    println!("请求数据: {}", first);
                    for line in rest {
                        println!("          {}", line);
                    }
                }
            } else {
                writeln!("请求数据: None");
            }
            if let Some(ref resp) = event.resp_body {
                let display = if resp.len() > DEFAULT_MAX_PAYLOAD_SIZE {
                    &resp[..DEFAULT_MAX_PAYLOAD_SIZE]
                } else {
                    resp
                };
                let lines: Vec<&str> = display.lines().collect();
                if let Some((first, rest)) = lines.split_first() {
                    writeln!("响应数据: {}", first);
                    for line in rest {
                        writeln!("          {}", line);
                    }
                }
            } else {
                writeln!("响应数据: None");
            }
            writeln!(w, "请求大小: {}", format_size(event.request_size));
            writeln!(w, "响应大小: {}", format_size(event.response_size));
            writeln!(w, "处理时长: {}", duration_str);
            writeln!(w, "-------------------------------------------------------");
        } else {
            writeln!(
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
