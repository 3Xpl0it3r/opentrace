rm -f crates/opentrace-bpf/src/bpf/*.skel.rs
rm  ./target/debug/opentrace-*
cargo build  --package opentrace-bpf
cargo build  --package opentrace-kit
cargo build  --package opentrace-mcp
cargo build  --package opentrace-server
cargo build  --package opentrace-agent
cargo fmt
# sudo ./target/debug/opentrace-cli trace skbdrop -f "host 111.63.65.103 and tcp and port 80"
# pid=$1
# sudo ./target/debug/opentrace-cli  perf profile -p ${pid}
# sudo ./target/debug/opentrace-cli watch  http -p 2003 -v 
