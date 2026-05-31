---
layout: home

hero:
  name: OpenTrace
  text: 基于 eBPF 的 Linux 内核网络可观测工具
  tagline: 提供 skb 丢包追踪、CPU 性能剖析等能力，支持命令行与 MCP 服务两种调用方式
  image:
    src: /index.png
    alt: OpenTrace Logo
  actions:
    - theme: brand
      text: 快速开始
      link: /guide/quickstart
    - theme: alt
      text: GitHub
      link: https://github.com/3Xpl0it3r/opentrace

features:
  - icon: 🔍
    title: skb 丢包追踪
    details: 追踪内核网络栈中的 skb 丢包事件，支持按协议、地址、端口等条件过滤
  - icon: 📊
    title: CPU 性能剖析
    details: 基于 perf event 的 CPU 采样，支持内核栈和用户栈的符号解析
  - icon: 🌐
    title: 应用层协议解析
    details: 可插拔的协议解析框架，内置 HTTP/1.x 和 HTTP/2 支持，易于扩展自定义协议
  - icon: 🛠️
    title: 多种调用方式
    details: 支持 CLI 命令行和 MCP 服务两种调用方式，满足不同场景需求
  - icon: 🐳
    title: 容器感知
    details: 支持按容器 ID、容器名、Kubernetes Pod 名称进行过滤
  - icon: ⚡
    title: 高性能
    details: 基于 eBPF 技术，在内核态完成数据采集和过滤，最小化性能开销
---

## 快速安装

```bash
# 克隆项目
git clone https://github.com/3Xpl0it3r/opentrace.git
cd opentrace

# 构建
make build

# 运行
sudo ./target/debug/opentrace-cli trace skbdrop
```

## 环境要求

- Linux 内核 ≥ 3.10（支持 kprobe / eBPF）
- Rust ≥ 1.95.0
- 运行时需 root 或等价的 BPF / 内核追踪权限
