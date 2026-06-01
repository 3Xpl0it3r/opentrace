// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.
// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.
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

use super::Config;
use super::Event;

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
