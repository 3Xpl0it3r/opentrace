// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.

use std::mem::MaybeUninit;

use libbpf_rs::skel::{OpenSkel as _, SkelBuilder};
use libbpf_rs::{MapCore, MapFlags, OpenObject, PerfBufferBuilder};

use crate::bpf::skbdrop::{SkbdropSkel, SkbdropSkelBuilder};
use crate::env;
use crate::errors::EbpfError;
use crate::exporters::{Exporter, helper::load_and_dispatch};
use crate::probes::Registry as ProbeRegistry;
use crate::skeleton::with_custom_btf_open_opts;

use crate::collectors::macros::{attach_kprobe, attach_kretprobe, define_collector};

use super::config::Config;
use super::event::Event;

// 在

const CONFIG_KEY: u8 = 0;
/// 5.16+ 内核 drop reason。
const KFREE_SKB_REASON: &str = "kfree_skb_reason";
///  5.16以下版本内核
const KFREE_SKB_FALLBACK: &str = "__kfree_skb";
/// 在4.19/4.18版本内核上测试发现iptables的drop和reject包并没有被__kfree_skb抓到，所以在nf_hook_slow上hook了下
/// 但是为了在z正常__kfree_skb能抓到iptables drop的包的内核上有重复，所以对这个hook做了内科版本限制
const NF_HOOK_SLOW: &str = "nf_hook_slow";

// 创建Collector结构体，并且自动实现Collector trait
define_collector!(Collector, SkbdropSkel);

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
                load_and_dispatch::<Event, _>(data, &mut exporter);
            })
            .build()?;

        Ok(Self {
            probe_registry: registry,
            skel,
            perf_buffer,
            _links: Vec::new(),
        })
    }
    fn do_attach_probes(&mut self) -> Result<(), EbpfError> {
        let mut attached = 0usize;

        // 1) kfree_skb 系列：覆盖 TCP 栈/qdisc/驱动等非 netfilter drop
        let kfree_target = if self.probe_registry.kprobe_is_available(KFREE_SKB_REASON) {
            Some(KFREE_SKB_REASON)
        } else if self.probe_registry.kprobe_is_available(KFREE_SKB_FALLBACK) {
            Some(KFREE_SKB_FALLBACK)
        } else {
            None
        };
        if let Some(name) = kfree_target {
            attach_kprobe!(self, kp_kfree_skb, name);
            println!("kprobe attached: {}", name);
            attached += 1;
        }

        // 低于5.0版本的内核为了方式iptables
        // drop和reject的包无法在kfree_skb上捕获到，因此额外挂在nf_hook_slow的包
        let kver = env::kernel_version();
        let need_nf_hook = kver < (5, 0);
        if need_nf_hook && self.probe_registry.kprobe_is_available(NF_HOOK_SLOW) {
            attach_kprobe!(self, kp_nf_hook_slow, NF_HOOK_SLOW);
            attach_kretprobe!(self, kp_nf_hook_slow, NF_HOOK_SLOW);

            println!(
                "kprobe attached: {} (entry+ret, kernel {}.{} < 5.0)",
                NF_HOOK_SLOW, kver.0, kver.1
            );
            attached += 1;
        }

        if attached == 0 {
            return Err(EbpfError::ProbeNotFound(format!(
                "none of [{}, {}, {}] available",
                KFREE_SKB_REASON, KFREE_SKB_FALLBACK, NF_HOOK_SLOW
            )));
        }
        Ok(())
    }
}
