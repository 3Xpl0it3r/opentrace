// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.
use std::collections::BTreeMap;
use std::mem::MaybeUninit;
use std::os::fd::{AsRawFd, OwnedFd};
use std::{mem, slice};

use libbpf_rs::skel::{OpenSkel, SkelBuilder};
use libbpf_rs::{
    self, Link, MapCore, MapFlags, OpenObject, PerfBuffer, PerfBufferBuilder, PrintLevel,
    libbpf_sys,
};
use libc::setgid;
use serde::{Deserialize, Serialize};

use crate::bpf::perf_profile::{self, PerfProfileSkel, PerfProfileSkelBuilder};
use crate::collectors::Collector as CollectorTrait;
use crate::probes::Registry as ProbeRegistry;
use crate::symbols::{SymbolTable, new_kernel_symbol};
use crate::types::process::ProcessInfo;
use crate::utils::syscall::{self as syscall_utils, PerfEventFdBuilder};
use crate::{EbpfError, Exporter, utils};

use super::treestack::{StackTree, StackTreeNode};

const CONFIG_KEY: u8 = 0;
const TASK_COMM_LEN: usize = 16;

#[derive(Clone)]
#[repr(C)]
pub struct Event {
    process_info: ProcessInfo,
    pub(crate) kstack: [u64; 16],
    pub(crate) ustack: [u64; 16],
    pub(crate) kstack_sz: i64,
    pub(crate) ustack_sz: i64,
    timestamp: u64,
    cpu_id: u32,
}

// ebpf程序配置文件
#[derive(Default)]
pub struct Config {
    pub pid: i32,
    pub cpu: i32,
    pub group_id: i32,
}

#[repr(C)]
struct InnerConfig {
    pid: u32,
}

impl InnerConfig {
    fn from_config(config: &Config) -> Self {
        Self {
            pid: if config.pid > 0 { config.pid as u32 } else { 0 },
        }
    }

    fn as_bytes(&self) -> &[u8] {
        let ptr = self as *const Self as *const u8;
        let len = mem::size_of_val(self);
        unsafe { slice::from_raw_parts(ptr, len) }
    }
}

// perf profile event,
pub struct Collector<'a> {
    skel: perf_profile::PerfProfileSkel<'a>,
    probe_registry: &'a ProbeRegistry,
    /* perf_event_builder: PerfEventFdBuilder, */
    /// perf event fd, trans into OwnerFd, when Program is dropped, when pfd will also be dropped
    perf_buffer: PerfBuffer<'a>,
    pfd: OwnedFd,
    _links: Vec<Link>,
}

impl<'a> Collector<'a> {
    pub fn new(
        object: &'a mut MaybeUninit<OpenObject>,
        probe_registry: &'a ProbeRegistry,
        config: Config,
        mut exporter: impl Exporter<Event> + 'a,
    ) -> Result<Self, EbpfError> {
        let mut pfd_builder = PerfEventFdBuilder::default();
        pfd_builder.attach_pid(config.pid);
        pfd_builder.attach_cpu(config.cpu);
        let pfd = pfd_builder.build()?;

        let skel = PerfProfileSkelBuilder::default().open(object)?.load()?;
        let perf_buffer = PerfBufferBuilder::new(&skel.maps.perf_events)
            .sample_cb(move |_cpu: i32, data: &[u8]| {
                crate::exporter::load_and_dispatch(data, &mut exporter);
            })
            .build()?;

        Ok(Self {
            perf_buffer: perf_buffer,
            probe_registry: probe_registry,
            skel,
            _links: Vec::new(),
            /* perf_event_builder: pfd_builder, */
            pfd: pfd,
        })
    }
}

impl<'a> CollectorTrait for Collector<'a> {
    fn poll(&mut self, interval: std::time::Duration) -> Result<(), crate::EbpfError> {
        let _ = self.perf_buffer.poll(interval);
        Ok(())
    }

    fn attach_probe(&mut self) -> Result<(), crate::EbpfError> {
        let link = self
            .skel
            .progs
            .perf_profile_samples
            .attach_perf_event(self.pfd.as_raw_fd())?;
        self._links.push(link);
        Ok(())
    }
}

impl Serialize for Event {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        todo!()
    }
}

impl<'de> Deserialize<'de> for Event {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        todo!()
    }
}

// DefaultConsoleExporter[#TODO] (shoule add some comments )
pub struct DefaultConsoleExporter {
    kstack_tree: StackTree,
    ustack_tree: StackTree,
    kernel_symbols: SymbolTable,
}

impl Default for DefaultConsoleExporter {
    fn default() -> Self {
        Self {
            kernel_symbols: crate::symbol::new_kernel_symbol(),
            ..Default::default()
        }
    }
}

impl Exporter<Event> for DefaultConsoleExporter {
    fn dispatch(&mut self, event: Event) {
        self.kstack_tree.insert(&event.kstack);
        self.ustack_tree.insert(&event.ustack);
    }
}
