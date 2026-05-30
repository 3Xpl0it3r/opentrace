cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo llvm-cov --lcov --output-path lcov.info
cargo crap --lcov lcov.info
