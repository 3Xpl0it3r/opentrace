// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.

use std::mem::MaybeUninit;
use std::{mem, slice, time::Duration};

use libbpf_rs::skel::{OpenSkel as _, SkelBuilder};
use libbpf_rs::{Link, MapCore, MapFlags, OpenObject, PerfBuffer, PerfBufferBuilder};
use serde::ser::SerializeStruct;
use serde::{Deserialize, Serialize};

use crate::Exporter;
use crate::bpf::skbdrop::{SkbdropSkel, SkbdropSkelBuilder};
use crate::collectors::Collector as CollectorTrait;
use crate::errors::EbpfError;
use crate::format::StreamFormatter;
use crate::probes::Registry as ProbeRegistry;
use crate::skeleton::with_custom_btf_open_opts;
use crate::symbolizers::*;
use crate::utils::net as net_utils;

use crate::types::net::{AddrV4, AddrV6, L2Info, L3Info, L4Info, PktInfo};

const CONFIG_KEY: u8 = 0;
const KFREE_SKB_KPROBE: &str = "kfree_skb_reason";

#[derive(Clone, Copy)]
#[repr(C)]
pub struct Event {
    pub l2_info: L2Info,
    pub l3_info: L3Info,
    pub l4_info: L4Info,
    pub pkt_info: PktInfo,
    pub stack_size: i64,
    pub stack: [u64; 16],
    drop_reason: u8,
}

impl Serialize for Event {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let stack_len = if self.stack_size > 0 {
            ((self.stack_size as usize) / mem::size_of::<u64>()).min(self.stack.len())
        } else {
            0
        };
        let field_cnt = if stack_len > 0 { 4 } else { 3 };

        let mut state = serializer.serialize_struct("Event", field_cnt)?;

        state.serialize_field("l2", &self.l2_info)?;
        state.serialize_field("l3", &self.l3_info)?;
        state.serialize_field("l4", &self.l4_info)?;

        if stack_len == 0 {
            return state.end();
        }
        state.serialize_field("stack", &self.stack[..stack_len])?;
        state.end()
    }
}

impl<'de> Deserialize<'de> for Event {
    fn deserialize<D>(_deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        todo!()
    }
}

// 暴露给用户态的配置文件, 转换成InnerConfig 再传给ebpf
#[derive(Default, Debug)]
pub struct Config {
    pub any_addr: String,
    pub src_addr: String,
    pub dst_addr: String,
    pub pid: u32,
    pub netns: u32,
    pub eth_proto: u16,
    pub ip_proto: u16,
    pub any_port: u16,
    pub src_port: u16,
    pub dst_port: u16,
    pub custom_btf_path: Option<String>,
}

// 传递给内核态 eBPF 的配置，字段与 bpf/skbdrop.c 里的 struct config 一一对应。
#[repr(C)]
struct InnerConfig {
    any_addr: [u32; 4],
    src_addr: [u32; 4],
    dst_addr: [u32; 4],
    pid: u32,
    netns: u32,
    eth_proto: u16,
    ip_proto: u16,
    any_port: u16,
    src_port: u16,
    dst_port: u16,
    _pad: [u8; 6],
}

impl InnerConfig {
    fn as_bytes(&self) -> &[u8] {
        let ptr = self as *const Self as *const u8;
        let len = mem::size_of_val(self);
        unsafe { slice::from_raw_parts(ptr, len) }
    }
}

impl Config {
    fn into_inner(self) -> Result<InnerConfig, EbpfError> {
        Ok(InnerConfig {
            any_addr: net_utils::ipaddr_to_u128(&self.any_addr)?,
            src_addr: net_utils::ipaddr_to_u128(&self.src_addr)?,
            dst_addr: net_utils::ipaddr_to_u128(&self.dst_addr)?,
            pid: self.pid,
            netns: self.netns,
            eth_proto: self.eth_proto,
            ip_proto: self.ip_proto,
            any_port: self.any_port,
            src_port: self.src_port,
            dst_port: self.dst_port,
            _pad: [0; 6],
        })
    }
}

pub struct Collector<'a> {
    probe_registry: &'a ProbeRegistry,
    skel: SkbdropSkel<'a>,
    perf_buffer: PerfBuffer<'a>,
    _links: Vec<Link>,
}

impl<'a> Collector<'a> {
    pub fn new(
        object: &'a mut MaybeUninit<OpenObject>,
        registry: &'a ProbeRegistry,
        config: Config,
        mut exporter: impl Exporter<Event> + 'a,
    ) -> Result<Self, EbpfError> {
        let skel = match config.custom_btf_path {
            Some(ref custom_btf_path) => with_custom_btf_open_opts(custom_btf_path, |open_opts| {
                Ok(SkbdropSkelBuilder::default()
                    .open_opts(open_opts, object)?
                    .load()?)
            })?,
            None => SkbdropSkelBuilder::default().open(object)?.load()?,
        };

        skel.maps
            .config_map
            .update(
                &CONFIG_KEY.to_ne_bytes(),
                config.into_inner()?.as_bytes(),
                MapFlags::ANY,
            )
            .map_err(EbpfError::Libbpf)?;

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

impl CollectorTrait for Collector<'_> {
    fn poll(&mut self, interval: Duration) -> Result<(), EbpfError> {
        let _ = self.perf_buffer.poll(interval);
        Ok(())
    }

    fn attach_probe(&mut self) -> Result<(), EbpfError> {
        println!("kprobe attached");
        if !self.probe_registry.kprobe_is_available(KFREE_SKB_KPROBE) {
            return Err(EbpfError::ProbeNotFound(KFREE_SKB_KPROBE.into()));
        }
        let link = self
            .skel
            .progs
            .kp_kfree_skb
            .attach_kprobe(false, KFREE_SKB_KPROBE)
            .map_err(EbpfError::Libbpf)?;
        self._links.push(link);
        Ok(())
    }
}

pub struct DefaultFormatter<'a> {
    symbolizer: &'a dyn Symbolizer,
    source: Source<'a>,
}

impl<'a> StreamFormatter<Event> for DefaultFormatter<'a> {
    fn format<W: std::io::Write>(&self, w: &mut W, event: &Event) -> Result<(), std::io::Error> {
        use std::io::Write as _;

        let source = Source::Kernel;
        let sport = u16::from_be(event.l4_info.sport);
        let dport = u16::from_be(event.l4_info.dport);

        match event.l3_info.ip_version {
            4 => {
                writeln!(
                    w,
                    " {:<22}:{} {:<22}:{}",
                    AddrV4::from(event.l3_info.saddr),
                    sport,
                    AddrV4::from(event.l3_info.daddr),
                    dport,
                )?;
            }
            6 => {
                writeln!(
                    w,
                    " {:<22}:{} {:<22}:{}",
                    AddrV6::from(event.l3_info.saddr),
                    sport,
                    AddrV6::from(event.l3_info.daddr),
                    dport,
                )?;
            }
            _ => {
                writeln!(
                    w,
                    " {:<22} {:<22}",
                    format_args!("{}:{}", "0.0.0.0", sport),
                    format_args!("{}:{}", "0.0.0.0", dport),
                )?;
            }
        }

        if event.stack_size > 0 {
            let stk_cnt = (event.stack_size as usize) / mem::size_of::<u64>();
            for addr in event.stack[..stk_cnt.min(event.stack.len())].iter() {
                let symb = self.symbolizer.resolve(SymbolizeInput {
                    source: self.source.clone(),
                    addr: *addr,
                });
                writeln!(w, "        {}({})", symb.name, symb.offset)?;
            }
        }

        writeln!(w, "{}", "---+---".repeat(10))
    }
}

// 用于debug ，默认实现
pub struct DefaultConsoleExporter<'a> {
    formatter: DefaultFormatter<'a>,
}

impl<'a> DefaultConsoleExporter<'a> {
    pub fn new(symbolizer: &'a dyn Symbolizer) -> Self {
        Self {
            formatter: DefaultFormatter {
                symbolizer,
                source: Source::Kernel,
            },
        }
    }
}

impl Exporter<Event> for DefaultConsoleExporter<'_> {
    fn dispatch(&mut self, event: Event) {
        let mut stdout = std::io::stdout().lock();
        if let Err(err) = self.formatter.format(&mut stdout, &event) {
            eprintln!("failed to format skbdrop event: {err}");
        }
    }
}
