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
use crate::protocols::ProtoParser;
use crate::protocols::app_protos::HttpParser;
use crate::skeleton::with_custom_btf_open_opts;
use crate::types::net::Addr;
use crate::utils::cstring;
use crate::{EbpfError, Exporter, ProbeRegistry};

const CONFIG_KEY: u8 = 0;

// Storage[#TODO] (shoule add some comments )
/*
pid: {
     fd1: {
          remote_addr:  {
                id:
          }
     }

     fd2: {
          remote_addr:  xxx
     }
}

id1: {}

 */

// Event[#TODO] (shoule add some comments )
#[derive(Clone)]
#[repr(C)]
pub struct Event {
    pub buffer: [u8; 1024],
    pub timestamp: u64,
    pub size: u32,
}

// Config[#TODO] (shoule add some comments )
pub struct Config {
    pub custom_btf_path: Option<String>,
    pub pid: u32,
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

// Collector[#TODO] (shoule add some comments )
define_collector!(Collector, SocketTraceSkel);

impl<'a> Collector<'a> {
    pub fn new(
        object: &'a mut MaybeUninit<OpenObject>,
        registry: &'a ProbeRegistry,
        config: Config,
        mut exporter: impl Exporter<Event> + 'a,
    ) -> Result<Self, EbpfError> {
        let skel = match config.custom_btf_path {
            Some(ref custom_btf_path) => with_custom_btf_open_opts(custom_btf_path, |open_opts| {
                Ok(SocketTraceSkelBuilder::default()
                    .open_opts(open_opts, object)?
                    .load()?)
            })?,
            None => SocketTraceSkelBuilder::default().open(object)?.load()?,
        };
        let key: u8 = 1;
        skel.maps.config_map.update(
            &CONFIG_KEY.to_ne_bytes(),
            Into::<InnerConfig>::into(config).as_bytes(),
            MapFlags::ANY,
        );

        let perf_buffer = PerfBufferBuilder::new(&skel.maps.perf_events)
            .sample_cb(move |_cpu: i32, data: &[u8]| {
                crate::exporter::load_and_dispatch(data, &mut exporter);
            })
            .build()?;
        Ok(Self {
            probe_registry: registry,
            skel,
            perf_buffer,
            _links: Vec::new(),
        })
    }
}

impl<'a> Collector<'a> {
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

// Exporter[#TODO] (shoule add some comments )

pub struct DefaultExporter {
    proto_parser: HttpParser,
}

impl DefaultExporter {
    pub fn new(proto_parser: HttpParser) -> Self {
        Self { proto_parser }
    }
}

impl Exporter<Event> for DefaultExporter {
    fn dispatch(&mut self, event: Event) {
        let mut stdout = std::io::stdout().lock();
        let frame = self.proto_parser.parse(&event.buffer, event.size as usize);
        println!(
            "stream_id={} size={} method: {:?} url={}\n",
            frame.stream_id, event.size, frame.method, frame.url
        );
    }
}
