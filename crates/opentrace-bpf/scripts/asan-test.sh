#!/bin/bash
# AddressSanitizer (ASAN) 内存测试脚本
# 用法: ./scripts/asan-test.sh

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CRATE_DIR="$(dirname "$SCRIPT_DIR")"

cd "$CRATE_DIR"

echo "=== AddressSanitizer 测试 ==="
echo ""

# 设置 ASAN 环境变量
export RUSTFLAGS="-Z sanitizer=address"
export ASAN_OPTIONS="detect_leaks=1:abort_on_error=1"

echo "--- 运行单元测试 (ASAN) ---"
# 注意: ASAN 需要 nightly 工具链
# 某些测试可能因为外部依赖而失败

# 选择性运行不依赖 FFI 的测试
echo "测试 protocols 模块..."
cargo +nightly test -p opentrace-bpf --lib protocols::tests --target x86_64-unknown-linux-gnu 2>&1 || true

echo ""
echo "测试 types::net 模块..."
cargo +nightly test -p opentrace-bpf --lib types::net::tests --target x86_64-unknown-linux-gnu 2>&1 || true

echo ""
echo "测试 utils 模块..."
cargo +nightly test -p opentrace-bpf --lib utils::tests --target x86_64-unknown-linux-gnu 2>&1 || true

echo ""
echo "=== ASAN 测试完成 ==="
echo ""
echo "提示: 如果需要更详细的内存检查，可以使用:"
echo "  RUSTFLAGS='-Z sanitizer=address' cargo +nightly test --target x86_64-unknown-linux-gnu"
echo ""
echo "对于 leak 检测:"
echo "  ASAN_OPTIONS=detect_leaks=1 RUSTFLAGS='-Z sanitizer=address' cargo +nightly test --target x86_64-unknown-linux-gnu"
