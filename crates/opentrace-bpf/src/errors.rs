// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.

use std::net::AddrParseError;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum EbpfError {
    #[error("IO Error: {0}")]
    IO(#[from] std::io::Error),

    #[error("Libbpf Error: {0}")]
    Libbpf(#[from] libbpf_rs::Error),

    #[error("addr_parse Error: {0}")]
    ParseErr(#[from] AddrParseError),

    #[error("probes is not found Error: {0}")]
    ProbeNotFound(String),

    #[error("Other Error: {0}")]
    Other(String),
}

impl From<EbpfError> for String {
    fn from(error: EbpfError) -> Self {
        match error {
            EbpfError::IO(err) => err.to_string(),
            EbpfError::Libbpf(err) => err.to_string(),
            EbpfError::Other(err) => err,
            EbpfError::ParseErr(err) => err.to_string(),
            EbpfError::ProbeNotFound(err) => err,
        }
    }
}
