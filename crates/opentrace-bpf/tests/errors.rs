// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.

//! 集成测试：errors 模块
//!
//! 测试错误类型

use opentrace_bpf::EbpfError;

// ==================== Display 测试 ====================

#[test]
fn ebpf_error_io_display() {
    let err = EbpfError::IO(std::io::Error::new(std::io::ErrorKind::NotFound, "file not found"));
    assert_eq!(err.to_string(), "IO Error: file not found");
}

#[test]
fn ebpf_error_probe_not_found_display() {
    let err = EbpfError::ProbeNotFound("kprobe_missing".to_string());
    assert_eq!(err.to_string(), "probes is not found Error: kprobe_missing");
}

#[test]
fn ebpf_error_syscall_display() {
    let err = EbpfError::SyscallErr("permission denied".to_string());
    assert_eq!(err.to_string(), "Syscall Error: permission denied");
}

#[test]
fn ebpf_error_symbolize_display() {
    let err = EbpfError::SymbolizeError("symbol not found".to_string());
    assert_eq!(err.to_string(), "Symbolize Error: symbol not found");
}

#[test]
fn ebpf_error_other_display() {
    let err = EbpfError::Other("something went wrong".to_string());
    assert_eq!(err.to_string(), "Other Error: something went wrong");
}

// ==================== From 转换测试 ====================

#[test]
fn ebpf_error_from_io_error() {
    let io_err = std::io::Error::new(std::io::ErrorKind::Other, "test");
    let err: EbpfError = io_err.into();
    assert!(matches!(err, EbpfError::IO(_)));
}

// ==================== Into<String> 测试 ====================

#[test]
fn ebpf_error_probe_not_found_into_string() {
    let err = EbpfError::ProbeNotFound("test_probe".to_string());
    let s: String = err.into();
    assert_eq!(s, "test_probe");
}

#[test]
fn ebpf_error_syscall_into_string() {
    let err = EbpfError::SyscallErr("syscall failed".to_string());
    let s: String = err.into();
    assert_eq!(s, "syscall failed");
}

#[test]
fn ebpf_error_symbolize_into_string() {
    let err = EbpfError::SymbolizeError("symbol error".to_string());
    let s: String = err.into();
    assert_eq!(s, "symbol error");
}

#[test]
fn ebpf_error_other_into_string() {
    let err = EbpfError::Other("other error".to_string());
    let s: String = err.into();
    assert_eq!(s, "other error");
}
