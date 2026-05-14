// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.

use std::marker::PhantomData;
use std::time::Duration;

use opentrace_bpf::collector::Collector;
use opentrace_bpf::collector::cpu::{ProfileCollector, ProfileEvent, ProfileFoldedFormatter};
use opentrace_bpf::format::Formatter;
use opentrace_bpf::symbol::{self, SymbolResolver};
use opentrace_bpf::{Exporter, ProbeRegistry};
use serde::Serialize;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel};

use crate::errors::CliError;
use crate::options::perf::ProfileOptions;

pub async fn run(
    command: crate::options::perf::Subcommand,
    registry: &mut ProbeRegistry,
    object: &mut opentrace_bpf::CollectorObject,
) -> Result<(), CliError> {
    match command {
        crate::options::perf::Subcommand::Profile(options) => {
            run_as_profile(options, registry, object).await?;
        }
    }
    Ok(())
}

// 因此profile的数据量可能非常大，因此channelexporter采用了ubuntuedchannel
struct ChannelExporter<T, F, R> {
    event_tx: UnboundedSender<String>,
    formatter: F,
    resolver: R,
    _marked: PhantomData<T>,
}

impl<T: Send + Sized + Serialize + Clone, F: Formatter<T>, R: SymbolResolver>
    ChannelExporter<T, F, R>
{
    fn new(formatter: F, resolver: R) -> (Self, UnboundedReceiver<String>) {
        let (event_tx, event_rx) = unbounded_channel::<String>();
        (
            Self {
                event_tx,
                formatter,
                resolver,
                _marked: PhantomData,
            },
            event_rx,
        )
    }
}

impl<T: Send + Sized + Serialize + Clone, F: Formatter<T>, R: SymbolResolver> Exporter<T>
    for ChannelExporter<T, F, R>
{
    fn dispatch(&mut self, event: T) {
        let mut buffer = Vec::new();
        if self
            .formatter
            .format(&mut buffer, &event, &self.resolver)
            .is_err()
        {
            return;
        }

        let _ = self
            .event_tx
            .send(unsafe { String::from_utf8_unchecked(buffer) });
    }
}

async fn run_as_profile(
    options: ProfileOptions,
    registry: &mut ProbeRegistry,
    object: &mut opentrace_bpf::CollectorObject,
) -> Result<(), CliError> {
    let (exporter, mut event_rx) =
        ChannelExporter::new(ProfileFoldedFormatter, symbol::new_kernel_symbol());

    let mut collector = ProfileCollector::new(object, registry, options.into(), exporter)?;
    collector.attach_probe()?;

    let mut poll_interval = tokio::time::interval(Duration::from_millis(100));

    loop {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {
                break;
            }
            _ = poll_interval.tick() => {
                collector.poll(Duration::from_millis(0))?;
            }
            event = event_rx.recv() => {
                let Some(event) = event else {
                    break;
                };
                println!("{}", event);
            }
        }
    }

    Ok(())
}
