// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.
// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.

use std::time::Duration;

use opentrace_bpf::collector::Collector;
use opentrace_bpf::collector::cpu::{ProfileCollector, ProfileStackEvent, ProfileStackStorage};
use opentrace_bpf::exporter::SimpleUnboundChannelExporter;
use opentrace_bpf::symbol::{Source, SymbolizerProvider};

use crate::errors::CliError;
use crate::options::CliOptsCtx;
use crate::options::perf::{Language, ProfileOptions};

const KSTACK_FLAGS: u64 = 0xFFFFFFFF;

pub async fn run_as_profile(
    ctx: CliOptsCtx,
    options: ProfileOptions,
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

    let (exporter, mut event_rx) = SimpleUnboundChannelExporter::<ProfileStackEvent, _>::new();
    let mut collector = ProfileCollector::new(object, options.to_config(ctx), exporter)?;
    collector.attach_probe()?;

    let mut pre_handle_stack_storage = ProfileStackStorage::default();
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
                    let mut stack = event.0;
                    let mut kstack = event.1;
                    if !kstack.is_empty() {
                        stack.push(KSTACK_FLAGS);
                        stack.append(&mut kstack);
                    }
                    pre_handle_stack_storage.insert(stack);
                }
            }
        }
    }
    println!("采样完成，开始处理......");
    let finnal_stack_storage = pre_handle_stack_storage.migrate_into_new_tree(source, symbolizer);
    println!("{}", finnal_stack_storage);

    Ok(())
}
