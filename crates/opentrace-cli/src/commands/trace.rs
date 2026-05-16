// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.

use std::time::Duration;

use opentrace_bpf::ProbeRegistry;
use opentrace_bpf::collector::Collector;
use opentrace_bpf::collector::net::{
    SkbdropCollector, SkbdropConfig, SkbdropConsoleExpoter, SkbdropEvent,
    SkbdropEventDefaultFormatter,
};
use opentrace_bpf::symbol;

use crate::errors::CliError;
use crate::options::trace::SkbDropOptions;

pub async fn run(
    command: crate::options::trace::Subcommand,
    registry: &mut ProbeRegistry,
    object: &mut opentrace_bpf::CollectorObject,
) -> Result<(), CliError> {
    match command {
        crate::options::trace::Subcommand::SkbDrop(skb_drop_options) => {
            run_as_skbdrop(skb_drop_options, registry, object)?;
        }
    }
    Ok(())
}

pub fn run_as_skbdrop(
    options: SkbDropOptions,
    registry: &mut ProbeRegistry,
    object: &mut opentrace_bpf::CollectorObject,
) -> Result<(), CliError> {
    let symbolizer = symbol::SymbolizerRegistry::default();
    let mut collector = SkbdropCollector::new(
        object,
        registry,
        options.to_config().into(),
        SkbdropConsoleExpoter::new(&symbolizer),
    )?;

    collector.attach_probe()?;
    loop {
        let _ = collector.poll(Duration::from_millis(100));
    }

    Ok(())
}
