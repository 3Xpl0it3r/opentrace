// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.

use std::time::Duration;

use opentrace_bpf::ProbeRegistry;
use opentrace_bpf::collector::Collector;
use opentrace_bpf::collector::net::{
    SkbdropCollector, SkbdropConsoleExpoter,
};
use opentrace_bpf::symbol::{self, Source};

use crate::errors::CliError;
use crate::options::CliOptsCtx;
use crate::options::trace::SkbDropOptions;

pub async fn run(
    ctx: CliOptsCtx,
    command: crate::options::trace::Subcommand,
    _custom_btf_path: Option<String>,
    registry: &mut ProbeRegistry,
    object: &mut opentrace_bpf::CollectorObject,
) -> Result<(), CliError> {
    match command {
        crate::options::trace::Subcommand::SkbDrop(skb_drop_options) => {
            run_as_skbdrop(ctx, skb_drop_options, registry, object)?;
        }
    }
    Ok(())
}

pub fn run_as_skbdrop(
    ctx: CliOptsCtx,
    options: SkbDropOptions,
    registry: &mut ProbeRegistry,
    object: &mut opentrace_bpf::CollectorObject,
) -> Result<(), CliError> {
    let symbolizer_provider = symbol::SymbolizerProvider::default();
    let symbolizer = symbolizer_provider.get_symbolizer(&Source::Kernel);

    let mut collector = SkbdropCollector::new(
        object,
        registry,
        options.to_config(ctx).into(),
        SkbdropConsoleExpoter::new(symbolizer),
    )?;

    collector.attach_probe()?;
    loop {
        let _ = collector.poll(Duration::from_millis(100));
    }
}
