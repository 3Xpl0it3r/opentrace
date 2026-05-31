![CI](https://img.shields.io/github/actions/workflow/status/3Xpl0it3r/opentrace/ci.yaml?label=CI&logo=github)
![Crap](https://img.shields.io/github/actions/workflow/status/3Xpl0it3r/opentrace/crap.yaml?label=Crap&logo=github)
![Docker](https://img.shields.io/badge/Docker-Pending-lightgrey?logo=docker)
![Security Audit](https://img.shields.io/github/actions/workflow/status/3Xpl0it3r/opentrace/audit.yaml?label=Security%20Audit&logo=github)
![GitHub commit activity](https://img.shields.io/github/commit-activity/m/3Xpl0it3r/opentrace?logo=github)
![GitHub last commit](https://img.shields.io/github/last-commit/3Xpl0it3r/opentrace?logo=github)

<p align="center">
  <img src="images/logo.png" alt="OpenTrace Logo" width="200">
</p>

# opentrace

[![Documentation](https://img.shields.io/badge/docs-VitePress-blue?logo=vitepress)](https://3Xpl0it3r.github.io/opentrace/)

基于 eBPF 的网络/CPU/内存/IO 全内核可观测工具，提供 skb 丢包追踪、CPU 性能剖析等能力，支持命令行与 MCP 服务两种调用方式。

📖 **[完整文档](https://3Xpl0it3r.github.io/opentrace/)**

## 核心功能

### skb 丢包追踪

追踪内核网络栈中的 skb 丢包事件，支持按协议、地址、端口等条件过滤：

```bash
sudo opentrace-cli trace skbdrop -f "tcp port 80"
```

### CPU 性能剖析

基于 perf event 的 CPU 采样，支持内核栈和用户栈的符号解析：

```bash
sudo opentrace-cli perf profile --pid 1234
```

### 应用层协议解析

可插拔的协议解析框架，内置 HTTP/1.x 和 HTTP/2 支持，易于扩展自定义协议。

### MCP 服务

支持通过 MCP (Model Context Protocol) 服务调用，便于集成到 AI 工具链：

```bash
sudo opentrace-server
```

## 技术架构

```
┌─────────────────────────────────────────────────────────────┐
│                      opentrace-cli                          │
│                   (用户命令行入口)                           │
└─────────────────────────┬───────────────────────────────────┘
                          │
┌─────────────────────────▼───────────────────────────────────┐
│                      opentrace-bpf                          │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────────┐  │
│  │  Collectors   │  │  Exporters   │  │    Formatters    │  │
│  │  (eBPF采集)   │  │  (数据导出)   │  │   (数据格式化)   │  │
│  └──────────────┘  └──────────────┘  └──────────────────┘  │
└─────────────────────────────────────────────────────────────┘
```

| 组件 | 作用 | 位置 |
|------|------|------|
| **Collector** | 用户态 eBPF 程序，负责挂载探针、采集内核数据 | `crates/opentrace-bpf/src/collectors/` |
| **Exporter** | 数据导出器，将采集的数据发送到目标（终端/ES/Kafka等） | `crates/opentrace-bpf/src/exporters/` |
| **Formatter** | 数据格式化器，将 Event 格式化为可读字符串 | `crates/opentrace-bpf/src/formatter.rs` |
| **Protocol** | 应用层协议解析器，将原始字节解析为结构化帧 | `crates/opentrace-bpf/src/protocols/` |

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

## 二次开发

详细的二次开发指南请参考文档：

- [开发概览](https://3Xpl0it3r.github.io/opentrace/guide/development/overview) - 架构、目录结构、核心 Trait
- [协议扩展](https://3Xpl0it3r.github.io/opentrace/guide/development/protocol-extension) - 自定义协议解析器开发
- [Exec Tracepoint 示例](https://3Xpl0it3r.github.io/opentrace/guide/examples/exec-tracepoint) - 完整的追踪器开发示例

## License

Apache-2.0
