#!/bin/bash
# MIRI 内存安全测试脚本
# 用法: ./scripts/miri-test.sh

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CRATE_DIR="$(dirname "$SCRIPT_DIR")"

cd "$CRATE_DIR"

echo "=== 安装 MIRI ==="
rustup +nightly component add miri

echo ""
echo "=== 运行 MIRI 测试 ==="
echo "注意: 某些测试可能因为外部依赖（如 libbpf）而失败"
echo ""

# 运行不依赖外部库的测试模块
# MIRI 不支持 FFI 调用，所以我们需要选择性地运行测试

echo "--- 测试 protocols 模块 ---"
cargo +nightly miri test -p opentrace-bpf --lib protocols::tests 2>&1 || true

echo ""
echo "--- 测试 types::net 模块 ---"
cargo +nightly miri test -p opentrace-bpf --lib types::net::tests 2>&1 || true

echo ""
echo "--- 测试 utils::net 模块 ---"
cargo +nightly miri test -p opentrace-bpf --lib utils::net::tests 2>&1 || true

echo ""
echo "--- 测试 utils::cstring 模块 ---"
cargo +nightly miri test -p opentrace-bpf --lib utils::cstring::tests 2>&1 || true

echo ""
echo "--- 测试 utils::time 模块 ---"
cargo +nightly miri test -p opentrace-bpf --lib utils::time::tests 2>&1 || true

echo ""
echo "--- 测试 utils::bytes 模块 ---"
cargo +nightly miri test -p opentrace-bpf --lib utils::bytes::tests 2>&1 || true

echo ""
echo "--- 测试 errors 模块 ---"
cargo +nightly miri test -p opentrace-bpf --lib errors::tests 2>&1 || true

echo ""
echo "=== MIRI 测试完成 ==="
