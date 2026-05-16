rm -f crates/opentrace-bpf/src/bpf/*.skel.rs
cargo build  --package opentrace-cli
cargo build  --package opentrace-server
# sudo ./target/debug/opentrace-cli trace skbdrop -f "host 111.63.65.103 and tcp and port 80"
# sudo ./target/debug/opentrace-cli  perf profile
