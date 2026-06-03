#!/bin/bash
# 综合内存安全测试脚本
# 用法: ./scripts/memory-test.sh [miri|asan|valgrind|all]

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CRATE_DIR="$(dirname "$SCRIPT_DIR")"

cd "$CRATE_DIR"

run_miri() {
    echo "=== 运行 MIRI 测试 ==="
    echo "MIRI 可以检测:"
    echo "  - 未定义行为 (UB)"
    echo "  - 内存越界访问"
    echo "  - 使用未初始化内存"
    echo "  - 数据竞争 (实验性)"
    echo ""

    rustup +nightly component add miri 2>/dev/null || true

    # 设置 MIRI flags
    export MIRIFLAGS="-Zmiri-disable-isolation -Zmiri-tree-borrows"

    echo "--- 测试 protocols::http ---"
    cargo +nightly miri test -p opentrace-bpf --lib protocols::http::tests 2>&1 || true

    echo ""
    echo "--- 测试 protocols::ether ---"
    cargo +nightly miri test -p opentrace-bpf --lib protocols::ether::tests 2>&1 || true

    echo ""
    echo "--- 测试 protocols::inet ---"
    cargo +nightly miri test -p opentrace-bpf --lib protocols::inet::tests 2>&1 || true

    echo ""
    echo "--- 测试 types::net ---"
    cargo +nightly miri test -p opentrace-bpf --lib types::net::tests 2>&1 || true

    echo ""
    echo "--- 测试 types::process ---"
    cargo +nightly miri test -p opentrace-bpf --lib types::process::tests 2>&1 || true

    echo ""
    echo "--- 测试 utils::net ---"
    cargo +nightly miri test -p opentrace-bpf --lib utils::net::tests 2>&1 || true

    echo ""
    echo "--- 测试 utils::cstring ---"
    cargo +nightly miri test -p opentrace-bpf --lib utils::cstring::tests 2>&1 || true

    echo ""
    echo "--- 测试 utils::time ---"
    cargo +nightly miri test -p opentrace-bpf --lib utils::time::tests 2>&1 || true

    echo ""
    echo "--- 测试 utils::bytes ---"
    cargo +nightly miri test -p opentrace-bpf --lib utils::bytes::tests 2>&1 || true

    echo ""
    echo "--- 测试 errors ---"
    cargo +nightly miri test -p opentrace-bpf --lib errors::tests 2>&1 || true

    echo ""
    echo "=== MIRI 测试完成 ==="
}

run_asan() {
    echo "=== 运行 AddressSanitizer 测试 ==="
    echo "ASAN 可以检测:"
    echo "  - 堆缓冲区溢出"
    echo "  - 栈缓冲区溢出"
    echo "  - 使用释放后的内存"
    echo "  - 内存泄漏"
    echo ""

    export RUSTFLAGS="-Z sanitizer=address"
    export ASAN_OPTIONS="detect_leaks=1:halt_on_error=0"

    echo "--- 运行单元测试 (ASAN) ---"
    cargo +nightly test -p opentrace-bpf --lib --target x86_64-unknown-linux-gnu 2>&1 || true

    echo ""
    echo "=== ASAN 测试完成 ==="
}

run_valgrind() {
    echo "=== 运行 Valgrind 测试 ==="
    echo "Valgrind 可以检测:"
    echo "  - 内存泄漏"
    echo "  - 未初始化内存使用"
    echo "  - 无效内存访问"
    echo ""

    if ! command -v valgrind &> /dev/null; then
        echo "错误: 未安装 valgrind"
        echo "请运行: sudo apt-get install valgrind (Ubuntu/Debian)"
        echo "        brew install valgrind (macOS)"
        return 1
    fi

    echo "--- 运行单元测试 (Valgrind) ---"
    cargo test -p opentrace-bpf --lib --no-run 2>&1

    # 找到测试二进制文件
    TEST_BINARY=$(find target/debug/deps -name "opentrace_bpf-*" -type f -executable | head -1)

    if [ -n "$TEST_BINARY" ]; then
        echo "测试二进制: $TEST_BINARY"
        valgrind --leak-check=full --show-leak-kinds=all --track-origins=yes "$TEST_BINARY" 2>&1 || true
    else
        echo "警告: 未找到测试二进制文件"
    fi

    echo ""
    echo "=== Valgrind 测试完成 ==="
}

# 主程序
case "${1:-all}" in
    miri)
        run_miri
        ;;
    asan)
        run_asan
        ;;
    valgrind)
        run_valgrind
        ;;
    all)
        echo "=== 运行所有内存安全测试 ==="
        echo ""
        run_miri
        echo ""
        echo "=========================================="
        echo ""
        run_asan
        echo ""
        echo "=========================================="
        echo ""
        run_valgrind
        ;;
    *)
        echo "用法: $0 [miri|asan|valgrind|all]"
        echo ""
        echo "选项:"
        echo "  miri      - 运行 MIRI 测试 (推荐，最全面)"
        echo "  asan      - 运行 AddressSanitizer 测试"
        echo "  valgrind  - 运行 Valgrind 测试"
        echo "  all       - 运行所有测试 (默认)"
        exit 1
        ;;
esac
