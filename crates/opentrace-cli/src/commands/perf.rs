// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.

mod profile;

use opentrace_bpf::ProbeRegistry;

use crate::errors::CliError;
use crate::options::CliOptsCtx;

pub async fn run(
    ctx: CliOptsCtx,
    command: crate::options::perf::Subcommand,
    _registry: &mut ProbeRegistry,
    object: &mut opentrace_bpf::CollectorObject,
) -> Result<(), CliError> {
    match command {
        crate::options::perf::Subcommand::Profile(options) => {
            profile::run_as_profile(ctx, options, object).await?;
        }
    }
    Ok(())
}
