# 简介

OpenTrace 是一个基于 eBPF 的 Linux 内核可观测工具，提供 skb 丢包追踪、CPU 性能剖析等能力。

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

| 组件 | 作用 |
|------|------|
| **Collector** | 用户态 eBPF 程序，负责挂载探针、采集内核数据 |
| **Exporter** | 数据导出器，将采集的数据发送到目标（终端/ES/Kafka等） |
| **Formatter** | 数据格式化器，将 Event 格式化为可读字符串 |
| **Protocol** | 应用层协议解析器，将原始字节解析为结构化帧 |

## 下一步

- [快速开始](/guide/quickstart) - 安装和运行 OpenTrace
- [CLI 命令](/guide/cli) - 了解 CLI 的使用方法
- [开发概览](/guide/development/overview) - 参与 OpenTrace 的开发
