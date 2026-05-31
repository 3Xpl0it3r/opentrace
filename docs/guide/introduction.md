# 简介

OpenTrace 是一个基于 eBPF 的可扩展 Linux 内核可观测平台，提供网络诊断、CPU 性能剖析、IO 诊断等核心能力，支持命令行与 MCP 服务两种调用方式，具备可扩展的应用层协议解析框架，允许用户自定义开发协议解析器。

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
sudo opentrace-mcp
```

## 技术架构

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                              用户入口层                                      │
│                                                                             │
│  ┌───────────────┐      ┌─────────────────────────┐  ┌───────────────────┐ │
│  │ opentrace-cli │      │     opentrace-mcp        │  │ opentrace-agent   │ │
│  │  (命令行工具)  │      │  ┌──────┐  ┌─────────┐  │  │ ┌──────┐ ┌─────┐ │ │
│  │               │      │  │ MCP  │  │ Server  │  │  │ │Agent │ │Server│ │ │
│  │               │      │  │(数据)│  │ (HTTP)  │  │  │ │(数据)│ │(HTTP)│ │ │
│  └───────┬───────┘      │  └──┬───┘  └─────────┘  │  │ └──┬───┘ └─────┘ │ │
│          │              └─────┼────────────────────┘  └────┼─────────────┘ │
└──────────┼────────────────────┼────────────────────────────┼───────────────┘
           │                    │                            │
           │                    │                            │
           ▼                    ▼                            ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                              核心能力层                                      │
│                              opentrace-bpf                                  │
│    ┌──────────────┐      ┌──────────────┐      ┌──────────────┐            │
│    │  Collectors   │      │  Exporters   │      │  Formatters  │            │
│    │  (eBPF采集)   │      │  (数据导出)  │      │  (数据格式化) │            │
│    └──────────────┘      └──────────────┘      └──────────────┘            │
└─────────────────────────────────────────────────────────────────────────────┘

注：opentrace-agent 待实现，架构与 opentrace-mcp 相同
```

| 组件 | 作用 |
|------|------|
| **opentrace-cli** | 命令行工具，直接调用 opentrace-bpf |
| **opentrace-mcp** | MCP 服务，数据层直接调用 opentrace-bpf，通过 opentrace-server 提供 HTTP 服务 |
| **opentrace-agent** | Agent 服务（待实现），架构同 opentrace-mcp |
| **opentrace-server** | 通用 HTTP 服务器框架，为 mcp/agent 提供 HTTP 服务 |
| **Collector** | 用户态 eBPF 程序，负责挂载探针、采集内核数据 |
| **Exporter** | 数据导出器，将采集的数据发送到目标（终端/ES/Kafka等） |
| **Formatter** | 数据格式化器，将 Event 格式化为可读字符串 |
| **Protocol** | 应用层协议解析器，将原始字节解析为结构化帧 |

## 下一步

- [快速开始](/guide/quickstart) - 安装和运行 OpenTrace
- [CLI 命令](/guide/cli) - 了解 CLI 的使用方法
- [开发概览](/guide/development/overview) - 参与 OpenTrace 的开发
