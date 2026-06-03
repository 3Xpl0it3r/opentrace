// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.

use std::time::Duration;

use opentrace_bpf::ProbeRegistry;
use opentrace_bpf::collectors::Collector;
use opentrace_bpf::collectors::net::{SkbdropCollector, SkbdropEventDefaultFormatter};
use opentrace_bpf::exporters::StreamWriterExpoter;
use opentrace_bpf::symbolizers::{self, Source};

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

fn run_as_skbdrop(
    ctx: CliOptsCtx,
    options: SkbDropOptions,
    registry: &mut ProbeRegistry,
    object: &mut opentrace_bpf::CollectorObject,
) -> Result<(), CliError> {
    let symbolizer_provider = symbolizers::SymbolizerProvider::default();
    let symbolizer = symbolizer_provider.get_symbolizer(&Source::Kernel);

    let mut collector = SkbdropCollector::new(
        object,
        registry,
        options.to_config(ctx).into(),
        StreamWriterExpoter::new(
            std::io::stdout(),
            SkbdropEventDefaultFormatter::new(symbolizer),
        ),
    )?;

    collector.attach_probe()?;
    loop {
        let _ = collector.poll(Duration::from_millis(100));
    }
}
