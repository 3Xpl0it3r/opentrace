// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.
use std::mem::MaybeUninit;
use std::os::fd::{AsRawFd, OwnedFd};
use std::{mem, slice};

use libbpf_rs::skel::{OpenSkel, SkelBuilder};
use libbpf_rs::{Link, MapCore, MapFlags, OpenObject, PerfBuffer, PerfBufferBuilder};
use serde::{Deserialize, Serialize};

use crate::bpf::perf_profile::{self, PerfProfileSkel, PerfProfileSkelBuilder};
use crate::collectors::Collector as CollectorTrait;
use crate::probes::Registry as ProbeRegistry;
use crate::types::process::ProcessInfo;
use crate::utils::syscall::PerfEventFdBuilder;
use crate::{EbpfError, Exporter};

const CONFIG_KEY: u8 = 0;
const TASK_COMM_LEN: usize = 16;
//采样最大栈深度
const SAMPLE_STACK_DEPTH: usize = 6;

#[derive(Clone)]
#[repr(C)]
pub struct Event {
    process_info: ProcessInfo,
    pub kstack: [u64; 16],
    pub ustack: [u64; 16],
    pub kstack_sz: i64,
    pub ustack_sz: i64,
    pub timestamp: u64,
    pub cpu_id: u32,
}

impl Event {
    fn stack_count(size: i64, max: usize) -> usize {
        if size <= 0 {
            return 0;
        }
        ((size as usize) / mem::size_of::<u64>()).min(max)
    }

    #[inline]
    pub fn stack_size(&self) -> (usize, usize) {
        (
            Self::stack_count(self.ustack_sz, SAMPLE_STACK_DEPTH as usize),
            Self::stack_count(self.kstack_sz, SAMPLE_STACK_DEPTH as usize),
        )
    }
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

impl From<Config> for InnerConfig {
    fn from(config: Config) -> Self {
        Self {
            pid: if config.pid > 0 { config.pid as u32 } else { 0 },
        }
    }
}

impl InnerConfig {
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
        if config.pid > 0 {
            pfd_builder.attach_pid(config.pid);
        }
        pfd_builder.attach_cpu(config.cpu);
        let pfd = pfd_builder.build()?;

        let skel = PerfProfileSkelBuilder::default().open(object)?.load()?;
        let perf_buffer = PerfBufferBuilder::new(&skel.maps.perf_events)
            .sample_cb(move |_cpu: i32, data: &[u8]| {
                crate::exporter::load_and_dispatch(data, &mut exporter);
            })
            .build()?;

        skel.maps.config_map.update(
            &CONFIG_KEY.to_ne_bytes(),
            InnerConfig::from(config).as_bytes(),
            MapFlags::ANY,
        )?;

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

// 提供默认的Expoter,
// profile的行为相对比较固定，只需要把用户/内核栈的栈地址发送给用户就可以了,没有多少的format操作
pub struct DefaultExporter {
    // 第一个Vec是用户栈, 第二个vec是内核栈
    event_tx: tokio::sync::mpsc::UnboundedSender<(Vec<u64>, Vec<u64>)>,
}

impl DefaultExporter {
    pub fn new() -> (
        Self,
        tokio::sync::mpsc::UnboundedReceiver<(Vec<u64>, Vec<u64>)>,
    ) {
        let (event_tx, event_rx) = tokio::sync::mpsc::unbounded_channel::<(Vec<u64>, Vec<u64>)>();
        (Self { event_tx }, event_rx)
    }
}
impl Exporter<Event> for DefaultExporter {
    fn dispatch(&mut self, event: Event) {
        let (ustk_size, kstk_size) = event.stack_size();
        let ustack = if ustk_size != 0 {
            let mut buffer = Vec::with_capacity(ustk_size);
            buffer.extend(&mut event.ustack[..ustk_size].iter().rev());
            buffer
        } else {
            vec![]
        };
        let kstack = if kstk_size != 0 {
            let mut buffer = Vec::with_capacity(kstk_size);
            buffer.extend(&mut event.kstack[..kstk_size].iter().rev());
            buffer
        } else {
            vec![]
        };

        let _ = self.event_tx.send((ustack, kstack));
    }
}
