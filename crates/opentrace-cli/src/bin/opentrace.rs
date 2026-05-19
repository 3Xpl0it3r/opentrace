// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.

use clap::Parser;

use opentrace_bpf::ProbeRegistry;

use opentrace_cli::errors::CliError;
use opentrace_cli::options::{self, CliOptions};

#[tokio::main]
async fn main() -> Result<(), CliError> {
    opentrace_bpf::env::setup_memlock_limit();
    let opts = CliOptions::parse();

    let mut probe_registry = ProbeRegistry::try_init()?;
    let mut object = opentrace_bpf::open_object_storage();
    let ctx = (&opts).into();

    match opts.commands {
        options::Command::Trace(subcommand) => {
            opentrace_cli::commands::trace::run(
                ctx,
                subcommand,
                opts.custom_btf_path,
                &mut probe_registry,
                &mut object,
            )
            .await
        }
        options::Command::Perf(subcommand) => {
            opentrace_cli::commands::perf::run(ctx, subcommand, &mut probe_registry, &mut object)
                .await
        }
    }
}
