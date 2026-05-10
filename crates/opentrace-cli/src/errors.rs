// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.

use thiserror::Error;

use opentrace_bpf::EbpfError;

#[derive(Debug, Error)]
pub enum CliError {
    #[error("IO Error: {0}")]
    IO(#[from] std::io::Error),

    #[error("Libbpf Error: {0}")]
    BpfErr(#[from] EbpfError),

    #[error("Argument Error: {0}")]
    ArgsErr(String),

    #[error("Other Error: {0}")]
    Other(String),
}
