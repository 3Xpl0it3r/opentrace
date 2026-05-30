use std::time::Duration;

use opentrace_bpf::ProbeRegistry;
use opentrace_bpf::collector::Collector;
use opentrace_bpf::collector::net::{SocketDefaultExporter, SocketTraceCollector};
use opentrace_bpf::protocol::appproto::HttpParser;

use crate::errors::CliError;
use crate::options::CliOptsCtx;
use crate::options::watch::HttpOptions;

// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.
//
pub async fn run(
    ctx: CliOptsCtx,
    command: crate::options::watch::Subcommand,
    registry: &mut ProbeRegistry,
    object: &mut opentrace_bpf::CollectorObject,
) -> Result<(), CliError> {
    match command {
        crate::options::watch::Subcommand::Http(elastic_options) => {
            run_watch_elastic(ctx, elastic_options, registry, object)?
        }
    }

    Ok(())
}

fn run_watch_elastic(
    ctx: CliOptsCtx,
    options: HttpOptions,
    registry: &mut ProbeRegistry,
    object: &mut opentrace_bpf::CollectorObject,
) -> Result<(), CliError> {
    let verbose = options.verbose;
    let mut collector = SocketTraceCollector::new(
        object,
        registry,
        options.to_config(ctx),
        SocketDefaultExporter::new(verbose),
        HttpParser::default(),
    )?;
    collector.attach_probe()?;

    loop {
        let _ = collector.poll(Duration::from_millis(100));
    }
}
