// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.
use std::mem;
use std::mem::MaybeUninit;
use std::os::fd::OwnedFd;

use libbpf_rs::skel::{OpenSkel, SkelBuilder};
use libbpf_rs::{Link, OpenObject, PerfBuffer, PerfBufferBuilder};

use crate::EbpfError;
use crate::bpf::perf_profile::{self, PerfProfileSkelBuilder};
use crate::collectors::Collector as CollectorTrait;
use crate::collectors::macros::attach_perf_event;
use crate::exporters::{Exporter, helper::load_and_dispatch};
use crate::skeleton::with_custom_btf_open_opts;
use crate::utils::procfs;
use crate::utils::syscall::PerfEventFdBuilder;

//采样最大栈深度
const SAMPLE_STACK_DEPTH: usize = 6;

#[derive(Clone)]
#[repr(C)]
pub struct Event {
    /* process_info: ProcessInfo, */
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
            Self::stack_count(self.ustack_sz, SAMPLE_STACK_DEPTH),
            Self::stack_count(self.kstack_sz, SAMPLE_STACK_DEPTH),
        )
    }
}

pub type StackEvent = (Vec<u64>, Vec<u64>);

impl From<Event> for StackEvent {
    fn from(event: Event) -> Self {
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
        (ustack, kstack)
    }
}

// 面向用户配置文件
#[derive(Default)]
pub struct Config {
    // pid 和 tid 只能二选一，设置了pid 就不能设置tid, 设置了tid就不能设置pid
    // 如果两个都不设置，则默认为0
    // 基于pid 过滤, 会捕获进程号是pid下面所有线程的event事件
    // 不填/none
    pub pid: i32,
    // 基于tid 过滤，可以指定一个或者多个
    // tid必须 > 0
    pub tids: Option<Vec<u32>>,
    // cpu -1 代表所有的cpu, >=0 代表指定的cpu
    pub cpu: i32,
    pub group_id: i32,

    pub custom_btf_path: Option<String>,
}

// perf profile event,
pub struct Collector<'a> {
    skel: perf_profile::PerfProfileSkel<'a>,
    /* perf_event_builder: PerfEventFdBuilder, */
    /// perf event fd, trans into OwnerFd, when Program is dropped, when pfd will also be dropped
    perf_buffer: PerfBuffer<'a>,
    pfds: Vec<OwnedFd>,
    _links: Vec<Link>,
}

impl<'a> Collector<'a> {
    pub fn new(
        object: &'a mut MaybeUninit<OpenObject>,
        config: Config,
        mut exporter: impl Exporter<Event> + 'a,
    ) -> Result<Self, EbpfError> {
        let pfds = build_perf_event_fds(config.cpu, config.pid)?;
        let skel = open_skel(object, config.custom_btf_path.as_deref())?;

        let perf_buffer = PerfBufferBuilder::new(&skel.maps.perf_events)
            .sample_cb(move |_cpu: i32, data: &[u8]| {
                load_and_dispatch::<Event, _>(data, &mut exporter);
            })
            .build()?;

        Ok(Self {
            perf_buffer,
            skel,
            _links: Vec::new(),
            pfds,
        })
    }
}

/// 根据 (cpu, pid) 解析出待挂载的 perf_event fd 列表。
///
/// pid >= 0 时按 `procfs` 枚举该进程的所有线程，每个 tid 一个 fd；
/// 否则只为 tid=-1（即整机所有线程）构建一个 fd。
fn build_perf_event_fds(cpu: i32, pid: i32) -> Result<Vec<OwnedFd>, EbpfError> {
    let mut pfd_builder = PerfEventFdBuilder::default();
    pfd_builder.attach_cpu(cpu);

    let tids = resolve_tids(pid);
    if tids.is_empty() {
        pfd_builder.attach_tid(-1_i32);
        return Ok(Vec::new());
    }

    let mut pfds = Vec::with_capacity(tids.len());
    for tid in tids {
        pfd_builder.attach_tid(tid as i32);
        pfds.push(pfd_builder.build()?);
    }
    Ok(pfds)
}

fn resolve_tids(pid: i32) -> Vec<u32> {
    if pid < 0 {
        return Vec::new();
    }
    procfs::thread_ids(pid as u32).unwrap_or_default()
}

fn open_skel<'a>(
    object: &'a mut MaybeUninit<OpenObject>,
    custom_btf_path: Option<&str>,
) -> Result<perf_profile::PerfProfileSkel<'a>, EbpfError> {
    match custom_btf_path {
        Some(path) => with_custom_btf_open_opts(path, |open_opts| {
            Ok(PerfProfileSkelBuilder::default()
                .open_opts(open_opts, object)?
                .load()?)
        }),
        None => Ok(PerfProfileSkelBuilder::default().open(object)?.load()?),
    }
}

impl<'a> CollectorTrait for Collector<'a> {
    fn poll(&mut self, interval: std::time::Duration) -> Result<(), crate::EbpfError> {
        let _ = self.perf_buffer.poll(interval);
        Ok(())
    }

    fn attach_probe(&mut self) -> Result<(), crate::EbpfError> {
        for pfd in self.pfds.iter() {
            attach_perf_event!(self, perf_profile_samples, pfd);
        }
        Ok(())
    }
}

// 提供默认的Expoter,
// profile的行为相对比较固定，只需要把用户/内核栈的栈地址发送给用户就可以了,没有多少的format操作
