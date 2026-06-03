// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.

//! 集成测试：env 模块
//!
//! 测试环境检测函数

use opentrace_bpf::env;

// ==================== default_custom_btf_path 测试 ====================

#[test]
fn default_custom_btf_path_returns_path_with_vmlinux_btf_name() {
    let path = env::default_custom_btf_path().unwrap();
    assert_eq!(path.file_name().unwrap(), "vmlinux.btf");
}

#[test]
fn default_custom_btf_path_is_absolute() {
    let path = env::default_custom_btf_path().unwrap();
    assert!(path.is_absolute());
}

// ==================== validate_btf_file 测试 ====================

// 注意：validate_btf_file 是私有函数，我们通过 check_btf_support 间接测试
// 或者使用单元测试（在 src/env.rs 中）

// ==================== kernel_version 测试 ====================

#[test]
fn kernel_version_returns_tuple() {
    let (major, minor) = env::kernel_version();
    // 在 Linux 上应该返回非零值，macOS 上可能返回 (0, 0)
    // 这里只验证函数不会 panic
    let _ = (major, minor);
}

// ==================== check_btf_support 测试 ====================

// 注意：check_btf_support 依赖内核 BTF 支持，测试结果取决于环境
// 这里只验证函数不会 panic
#[test]
fn check_btf_support_does_not_panic() {
    let _ = env::check_btf_support();
}
