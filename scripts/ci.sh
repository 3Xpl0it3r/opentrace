# 检查是否有编译问题
cargo build  --package opentrace-bpf
cargo build  --package opentrace-mcp
cargo build  --package opentrace-server
cargo build  --package opentrace-common

# 检查单元测试是否全通过
cargo test -p opentrace-bpf --features testing

# clippy检查
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo llvm-cov --lcov --output-path lcov.info
cargo crap --lcov lcov.info
