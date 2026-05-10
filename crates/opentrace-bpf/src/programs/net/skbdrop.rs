// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.

use std::{mem, slice, time::Duration};

use libbpf_rs::{Link, MapCore, MapFlags, PerfBuffer, PerfBufferBuilder, TracepointCategory};
use serde::ser::SerializeStruct;
use serde::{Deserialize, Serialize};

use crate::Exporter;
use crate::bpf::skbdrop::SkbdropSkel;
use crate::errors::EbpfError;
use crate::programs::EbpfProgram;
use crate::programs::probe_registry::ProbeRegistry;
use crate::utils::net as net_utils;
use crate::utils::os::{SymbolizedStack, cstr_to_string, kallsyms_by_addr};

use super::types::{AddrV4, AddrV6, L2Info, L3Info, L4Info, PktInfo, ProcessInfo};

const CONFIG_KEY: u8 = 0;
const KFREE_SKB_KPROBE: &str = "kfree_skb_reason";
const KFREE_SKB_TRACEPOINT: &str = "kfree_skb";
const KFREE_SKB_TRACEPOINT_ID: &str = "skb:kfree_skb";

#[derive(Clone, Copy)]
#[repr(C)]
pub struct Event {
    pub l2_info: L2Info,
    pub l3_info: L3Info,
    pub l4_info: L4Info,
    pub pkt_info: PktInfo,
    /* pub process_info: ProcessInfo, */
    pub stack_size: i64,
    pub stack: [u64; 16],
    pub drop_reason: u8,
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

pub struct Program<'a> {
    probe_registry: &'a ProbeRegistry,
    skel: SkbdropSkel<'a>,
    perf_buffer: PerfBuffer<'a>,
    _links: Vec<Link>,
}

impl<'a> Program<'a> {
    pub fn new(
        skel: SkbdropSkel<'a>,
        registry: &'a ProbeRegistry,
        config: Config,
        mut exporter: impl Exporter<Event> + 'a,
    ) -> Result<Self, EbpfError> {
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
                let event = exporter.load(data);
                exporter.handle(event);
            })
            .build()
            .map_err(EbpfError::Libbpf)?;

        Ok(Self {
            probe_registry: registry,
            skel,
            perf_buffer,
            _links: Vec::new(),
        })
    }
}

impl EbpfProgram for Program<'_> {
    fn poll(&mut self, duration: Duration) -> Result<(), EbpfError> {
        let _ = self.perf_buffer.poll(duration);
        Ok(())
    }

    fn attach_probe(&mut self) -> Result<(), EbpfError> {
        println!("kprobe attached");
        if !self.probe_registry.kprobe_is_available(KFREE_SKB_KPROBE) {
            return Err(EbpfError::ProbeNotFound(KFREE_SKB_KPROBE.into()));
        }
        let link = self.skel.progs.kp_kfree_skb.attach_kprobe(false, KFREE_SKB_KPROBE).map_err(EbpfError::Libbpf)?;
        self._links.push(link);
        Ok(())
    }
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
        state.serialize_field("stack", &SymbolizedStack(&self.stack[..stack_len]))?;
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

fn l3_addr_strings(l3_info: &L3Info) -> (String, String) {
    match l3_info.ip_version {
        4 => (
            AddrV4::from(l3_info.saddr).to_string(),
            AddrV4::from(l3_info.daddr).to_string(),
        ),
        6 => (
            AddrV6::from(l3_info.saddr).to_string(),
            AddrV6::from(l3_info.daddr).to_string(),
        ),
        _ => ("0.0.0.0".to_owned(), "0.0.0.0".to_owned()),
    }
}

// 用于debug ，默认实现
pub struct DefaultExporter;

impl Exporter<Event> for DefaultExporter {
    fn handle(&mut self, event: Event) {
        /* let pid = event.process_info.tgid_pid >> 32; */
        let (saddr, daddr) = l3_addr_strings(&event.l3_info);
        let sport = u16::from_be(event.l4_info.sport);
        let dport = u16::from_be(event.l4_info.dport);

        println!(
            " {:<22} {:<22}",
            /* cstr_to_string(&event.process_info.commond), */
            format!("{}:{}", saddr, sport),
            format!("{}:{}", daddr, dport),
        );
        if event.stack_size > 0 {
            let count = (event.stack_size as usize) / mem::size_of::<u64>();
            for addr in event.stack[..count.min(event.stack.len())].iter() {
                if let Some((syms, offset)) = kallsyms_by_addr(addr) {
                    println!("        {}({})", syms, offset);
                } else {
                    println!("{}", addr);
                }
            }
        }
        println!("{}", "---+---".repeat(10));
    }
}

fn debug_event(event: &Event){
        let (saddr, daddr) = l3_addr_strings(&event.l3_info);
        let sport = u16::from_be(event.l4_info.sport);
        let dport = u16::from_be(event.l4_info.dport);

        println!(
            " {:<22} {:<22}",
            /* cstr_to_string(&event.process_info.commond), */
            format!("{}:{}", saddr, sport),
            format!("{}:{}", daddr, dport),
        );
        if event.stack_size > 0 {
            let count = (event.stack_size as usize) / mem::size_of::<u64>();
            for addr in event.stack[..count.min(event.stack.len())].iter() {
                if let Some((syms, offset)) = kallsyms_by_addr(addr) {
                    println!("        {}({})", syms, offset);
                } else {
                    println!("{}", addr);
                }
            }
        }
        println!("{}", "---+---".repeat(10));

}
