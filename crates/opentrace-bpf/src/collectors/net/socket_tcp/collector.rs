// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.
// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.
//

use std::mem::MaybeUninit;

use libbpf_rs::skel::{OpenSkel, SkelBuilder};
use libbpf_rs::{MapCore, MapFlags, OpenObject, PerfBufferBuilder};

use crate::bpf::socket_tcp::{SocketTcpSkel, SocketTcpSkelBuilder};
use crate::collectors::macros::{
    attach_kprobe, attach_kretprobe, attach_multiple_tracepoints, attach_tracepoint,
    define_collector,
};
use crate::exporters::{Exporter, helper::load_and_dispath_with};
use crate::protocols::{ParsedFrame, ProtoParser};
use crate::skeleton::with_custom_btf_open_opts;
use crate::{EbpfError, ProbeRegistry};

use super::config::{Config, InnerConfig};
use super::event::{Event, InnerEvent};
use super::matcher::EventMatcher;

const CONFIG_KEY: u8 = 0;

define_collector!(Collector, SocketTcpSkel);

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
                Ok(SocketTcpSkelBuilder::default()
                    .open_opts(open_opts, object)?
                    .load()?)
            })?,
            None => SocketTcpSkelBuilder::default().open(object)?.load()?,
        };
        let verbose = config.verbose;
        let _ = skel.maps.config_map.update(
            &CONFIG_KEY.to_ne_bytes(),
            Into::<InnerConfig>::into(config).as_bytes(),
            MapFlags::ANY,
        );

        let mut event_cache = EventMatcher::new(proto_parser, verbose);
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
