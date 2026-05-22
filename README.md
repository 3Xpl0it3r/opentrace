![CI](https://img.shields.io/github/actions/workflow/status/3Xpl0it3r/opentrace/ci.yaml?label=CI)
![Crap](https://img.shields.io/github/actions/workflow/status/3Xpl0it3r/opentrace/crap.yaml?label=Crap)
![Docker](https://img.shields.io/badge/Docker-Pending-lightgrey)
![Security Audit](https://github.com/3Xpl0it3r/opentrace/actions/workflows/audit.yaml/badge.svg)
![GitHub commit activity](https://img.shields.io/github/commit-activity/m/3Xpl0it3r/opentrace)
![GitHub last commit](https://img.shields.io/github/last-commit/3Xpl0it3r/opentrace)

# opentrace

基于 eBPF 的 Linux 内核网络可观测工具，提供 skb 丢包追踪、CPU 性能剖析等能力，支持命令行与 MCP 服务两种调用方式。

## 环境要求

- Linux 内核 ≥ 3.10（支持 kprobe / eBPF）
- Rust ≥ 1.95.0
- 运行时需 root 或等价的 BPF / 内核追踪权限

## 安装

### 1. Rust 工具链

```bash
rustup install 1.95.0
rustup default 1.95.0
```

### 2. 系统依赖

Debian / Ubuntu：

```bash
sudo apt update
sudo apt install -y clang llvm bpftool libelf-dev zlib1g-dev build-essential
```

RHEL / CentOS / Fedora：

```bash
sudo dnf install -y clang llvm bpftool elfutils-libelf-devel zlib-devel gcc make
```

### 3. BTF 准备

检查内核是否已暴露 BTF：

```bash
ls -l /sys/kernel/btf/vmlinux
```

**存在该文件**：无需额外操作，直接进入构建步骤。

**不存在该文件**：使用项目内置流程生成本地 BTF：

```bash
make install-pahole       # 安装 pahole (dwarves)
make install-debuginfo    # 安装当前内核 debuginfo / dbgsym
make vmlinux              # 生成 scripts/include/vmlinux.{h,btf}
```

> Arch Linux 官方仓库不提供 `kernel-debuginfo`，需从 AUR 安装 `linux-debug` 或自行编译带调试符号的内核。

### 4. 构建

推荐使用 Makefile（自动探测 BTF 并按需准备依赖）：

```bash
make build       # debug 构建
make release     # release 构建
make info        # 输出发行版 / 内核 / 架构 / BTF 检测结果
```

也可直接使用 cargo（要求 BTF 已就绪或已手动放置 `scripts/include/vmlinux.{h,btf}`）：

```bash
cargo build                              # 默认构建 opentrace-server
cargo build --package opentrace-cli      # 构建 CLI 工具
```

### 5. 可选：Java 符号支持

对 Java 进程进行性能剖析需要 [jallsyms](https://github.com/3Xpl0it3r/jallsyms)：

```bash
git clone https://github.com/3Xpl0it3r/jallsyms.git
cd jallsyms && make && sudo make install
```

## 使用

### CLI

追踪 skb drop 事件：

```bash
sudo ./target/debug/opentrace-cli trace skbdrop
sudo ./target/debug/opentrace-cli trace skbdrop -f "tcp port 80"
sudo ./target/debug/opentrace-cli trace skbdrop -f "host 10.0.0.1 and tcp and port 443"
sudo ./target/debug/opentrace-cli trace skbdrop -f "src host 10.0.0.1 and dst port 443"
sudo ./target/debug/opentrace-cli trace skbdrop -f "udp port 53"
```

过滤表达式支持：`tcp` / `udp` / `icmp` / `host` / `src host` / `dst host` / `port` / `src port` / `dst port`。

命令行参数：

| 参数 | 说明 |
|---|---|
| `-f, --filter <EXPR>` | 过滤表达式 |
| `-i, --iface <IFACE>` | 指定网络接口 |
| `-p, --pid <PID>` | 按进程 ID 过滤 |
| `--pname <NAME>` | 按进程名过滤 |
| `--container-id <ID>` | 按容器 ID 过滤 |
| `--container-name <NAME>` | 按容器名过滤 |
| `--pod <POD>` | 按 Kubernetes Pod 名称过滤 |
| `-6, --v6` | 启用 IPv6 相关参数 |

### MCP 服务

启动服务（默认监听 `0.0.0.0:9999`）：

```bash
sudo ./target/debug/opentrace-server
# 或
sudo cargo run --package opentrace-server
```

健康检查：

```bash
curl http://127.0.0.1:9999/healthz
```

`skbdrop` 工具参数：

| 参数 | 说明 |
|---|---|
| `any_host` | 匹配源地址或目的地址 |
| `src_host` | 匹配源地址 |
| `dst_host` | 匹配目的地址 |
| `any_port` | 匹配源端口或目的端口 |
| `src_port` | 匹配源端口 |
| `dst_port` | 匹配目的端口 |
| `ip_proto` | IP 协议（`tcp` / `udp` / `icmp` 或协议号 `6` / `17` / `1`） |
| `eth_proto` | 以太网协议（`ipv4` / `ipv6` 或协议号 `0x0800` / `0x86DD`） |

调用后服务端等待 skb drop 事件并返回匹配项；若超时未捕获到事件则返回空结果。

## License

Apache-2.0
