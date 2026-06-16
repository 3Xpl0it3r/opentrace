// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.

use std::time::Duration;

use opentrace_bpf::ProbeRegistry;
use opentrace_bpf::collectors::Collector;
use opentrace_bpf::collectors::cpu::{ProfileCollector, ProfileEvent, ProfileStackStorage};
use opentrace_bpf::sinks::UnboundedChannelSink;
use opentrace_bpf::symbolizers::{Source, SymbolizerProvider};

use crate::errors::CliError;
use crate::options::CliOptsCtx;
use crate::options::perf::{Language, ProfileOptions};

const KSTACK_FLAGS: u64 = 0xFFFFFFFF;

pub async fn run_as_profile(
    ctx: CliOptsCtx,
    options: ProfileOptions,
    registry: &mut ProbeRegistry,
    object: &mut opentrace_bpf::CollectorObject,
) -> Result<(), CliError> {
    let mut symbolizer_provider = SymbolizerProvider::default();

    let source = match options.pid {
        Some(ref pid) if let Some(language) = options.language => match language {
            Language::Java => Source::JavaPid { pid: (*pid) as u32 }, // pid 已经在cli里面限制了大小
        },
        None => Source::Kernel,
        Some(ref pid) => Source::CPid { pid: (*pid) as u32 },
    };

    symbolizer_provider.register(&source);
    let symbolizer = symbolizer_provider.get_symbolizer(&source);

    let (sink, mut event_rx) = UnboundedChannelSink::<ProfileEvent>::new();
    let mut collector = ProfileCollector::new(object, options.to_config(ctx), sink)?;
    collector.attach_probe(registry)?;

    let mut stack_storage = ProfileStackStorage::default();
    let mut poll_interval = tokio::time::interval(Duration::from_millis(100));
    let sampling_timeout = tokio::time::sleep(Duration::from_secs(60));
    tokio::pin!(sampling_timeout);

    println!("开始采样.......");
    loop {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {
                break;
            }
            _ = &mut sampling_timeout => {
                break;
            }
            _ = poll_interval.tick() => {
                collector.poll(Duration::from_millis(0))?;
            }
            event = event_rx.recv() => {
                if let Some(event) = event {
                    let mut stack = event.ustack;
                    let mut kstack = event.kstack;
                    if !kstack.is_empty() {
                        stack.push(KSTACK_FLAGS);
                        stack.append(&mut kstack);
                    }
                    stack_storage.insert(stack);
                }
            }
        }
    }
    println!("采样完成，开始处理......");
    let merged = stack_storage.merged(source, symbolizer);
    println!("{}", merged);

    Ok(())
}
