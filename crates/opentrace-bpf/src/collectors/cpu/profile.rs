// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.
use std::mem;
use std::mem::MaybeUninit;
use std::os::fd::{AsRawFd, OwnedFd};

use libbpf_rs::skel::{OpenSkel, SkelBuilder};
use libbpf_rs::{Link, OpenObject, PerfBuffer, PerfBufferBuilder};

use crate::bpf::perf_profile::{self, PerfProfileSkelBuilder};
use crate::collectors::Collector as CollectorTrait;
use crate::skeleton::with_custom_btf_open_opts;
use crate::utils::procfs;
use crate::utils::syscall::PerfEventFdBuilder;
use crate::{EbpfError, Exporter};

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

/* impl Config {
    fn validate(config: &Config) -> Result<(), EbpfError> {
        if config.pid == -1 && config.cpu == -1 {
            return Err(EbpfError::ConfigErr(
                "ProfileConfig'field pid and cpu cann't be -1 at same time".into(),
            ));
        }
        Ok(())
    }
} */

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
        let mut pfd_builder = PerfEventFdBuilder::default();

        pfd_builder.attach_cpu(config.cpu);

        let tids = if config.pid >= 0 {
            procfs::thread_ids(config.pid as u32).unwrap_or(vec![])
        } else {
            vec![]
        };

        let mut pfds = Vec::new();
        if tids.is_empty() {
            pfd_builder.attach_tid(-1_i32);
        } else {
            for tid in tids {
                pfd_builder.attach_tid(tid as i32);
                pfds.push(pfd_builder.build()?);
            }
        }
        let skel = match config.custom_btf_path {
            Some(ref custom_btf_path) => with_custom_btf_open_opts(custom_btf_path, |open_opts| {
                Ok(PerfProfileSkelBuilder::default()
                    .open_opts(open_opts, object)?
                    .load()?)
            })?,
            None => PerfProfileSkelBuilder::default().open(object)?.load()?,
        };

        let perf_buffer = PerfBufferBuilder::new(&skel.maps.perf_events)
            .sample_cb(move |_cpu: i32, data: &[u8]| {
                crate::exporter::load_and_dispatch(data, &mut exporter);
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

impl<'a> CollectorTrait for Collector<'a> {
    fn poll(&mut self, interval: std::time::Duration) -> Result<(), crate::EbpfError> {
        let _ = self.perf_buffer.poll(interval);
        Ok(())
    }

    fn attach_probe(&mut self) -> Result<(), crate::EbpfError> {
        for pfd in self.pfds.iter() {
            let link = self
                .skel
                .progs
                .perf_profile_samples
                .attach_perf_event(pfd.as_raw_fd())?;
            self._links.push(link);
        }
        Ok(())
    }
}

// 提供默认的Expoter,
// profile的行为相对比较固定，只需要把用户/内核栈的栈地址发送给用户就可以了,没有多少的format操作

pub type StackEvent = (Vec<u64>, Vec<u64>);

pub struct DefaultExporter {
    // 第一个Vec是用户栈, 第二个vec是内核栈
    event_tx: tokio::sync::mpsc::UnboundedSender<StackEvent>,
}

impl DefaultExporter {
    pub fn new() -> (Self, tokio::sync::mpsc::UnboundedReceiver<StackEvent>) {
        let (event_tx, event_rx) = tokio::sync::mpsc::unbounded_channel::<StackEvent>();
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
