// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.

use std::{mem::MaybeUninit, time::Duration};

use clap::Parser;
use libc;

use opentrace_bpf::{
    EbpfProgram, ProbeRegistry,
    prog::net::{SkbdropDefaultExporter, SkbdropProgram},
    skel::{OpenSkel, SkbdropSkelBuilder, SkelBuilder},
};

use opentrace_cli::errors::CliError;
use opentrace_cli::options::{self, CliOptions};

fn setup_memlock_limit() {
    let rlim = libc::rlimit {
        rlim_cur: libc::RLIM_INFINITY,
        rlim_max: libc::RLIM_INFINITY,
    };
    let ret = unsafe { libc::setrlimit(libc::RLIMIT_MEMLOCK, &rlim) };
    if ret != 0 {
        eprintln!("Warning: Failed to remove memory lock limit (ret={})", ret);
    }
}

#[tokio::main]
async fn main() -> Result<(), CliError> {
    setup_memlock_limit();
    let opts = CliOptions::parse();

    let config = opts.to_config();
    config.validate()?;

    let probe_registry = ProbeRegistry::try_init()?;

    let mut open_project = MaybeUninit::uninit();
    let mut program: Box<dyn EbpfProgram> = match opts.commands {
        options::Command::Trace(subcommand) => match subcommand {
            options::trace::Subcommand::SkbDrop => {
                // Open load and verify BPF application
                let skel_builder = SkbdropSkelBuilder::default();
                let open_skel = skel_builder.open(&mut open_project).unwrap();
                // Load and verify BPF programs into kernel
                let skel = open_skel.load().unwrap();

                Box::new(SkbdropProgram::new(
                    skel,
                    &probe_registry,
                    config.into(),
                    SkbdropDefaultExporter,
                )?)
            }
        },
    };
    program.attach_probe()?;
    loop {
        let _ = program.poll(Duration::from_millis(100));
    }

    Ok(())
}
