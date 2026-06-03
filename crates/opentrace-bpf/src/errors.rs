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

    #[error("Syscall Error: {0}")]
    SyscallErr(String),

    #[error("Symbolize Error: {0}")]
    SymbolizeError(String),

    #[error("Config Error: {0}")]
    ConfigErr(String),

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
            EbpfError::SyscallErr(err) => err,
            EbpfError::SymbolizeError(err) => err,
            EbpfError::ConfigErr(_) => todo!(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn io_error_display() {
        let err = EbpfError::IO(std::io::Error::new(std::io::ErrorKind::NotFound, "file not found"));
        assert_eq!(err.to_string(), "IO Error: file not found");
    }

    #[test]
    fn parse_error_display() {
        let err: EbpfError = "invalid address".parse::<std::net::Ipv4Addr>().unwrap_err().into();
        assert!(err.to_string().contains("addr_parse Error"));
    }

    #[test]
    fn probe_not_found_display() {
        let err = EbpfError::ProbeNotFound("kprobe_missing".to_string());
        assert_eq!(err.to_string(), "probes is not found Error: kprobe_missing");
    }

    #[test]
    fn syscall_error_display() {
        let err = EbpfError::SyscallErr("permission denied".to_string());
        assert_eq!(err.to_string(), "Syscall Error: permission denied");
    }

    #[test]
    fn symbolize_error_display() {
        let err = EbpfError::SymbolizeError("symbol not found".to_string());
        assert_eq!(err.to_string(), "Symbolize Error: symbol not found");
    }

    #[test]
    fn other_error_display() {
        let err = EbpfError::Other("something went wrong".to_string());
        assert_eq!(err.to_string(), "Other Error: something went wrong");
    }

    #[test]
    fn io_error_from() {
        let io_err = std::io::Error::new(std::io::ErrorKind::Other, "test");
        let err: EbpfError = io_err.into();
        assert!(matches!(err, EbpfError::IO(_)));
    }

    #[test]
    fn probe_not_found_to_string() {
        let err = EbpfError::ProbeNotFound("test_probe".to_string());
        let s: String = err.into();
        assert_eq!(s, "test_probe");
    }

    #[test]
    fn syscall_error_to_string() {
        let err = EbpfError::SyscallErr("syscall failed".to_string());
        let s: String = err.into();
        assert_eq!(s, "syscall failed");
    }

    #[test]
    fn symbolize_error_to_string() {
        let err = EbpfError::SymbolizeError("symbol error".to_string());
        let s: String = err.into();
        assert_eq!(s, "symbol error");
    }

    #[test]
    fn other_error_to_string() {
        let err = EbpfError::Other("other error".to_string());
        let s: String = err.into();
        assert_eq!(s, "other error");
    }
}
