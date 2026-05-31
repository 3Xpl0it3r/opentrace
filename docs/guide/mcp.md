# MCP 服务

OpenTrace 支持通过 MCP (Model Context Protocol) 服务调用，便于集成到 AI 工具链。

## 架构说明

opentrace-mcp 的架构：
- **MCP 数据层**: 直接调用 opentrace-bpf 采集 eBPF 数据
- **HTTP 服务层**: 使用 opentrace-server 提供 HTTP 服务、TLS、认证等能力

```
┌─────────────────────────────────────────┐
│           opentrace-mcp                 │
│  ┌─────────────┐    ┌───────────────┐  │
│  │  MCP 工具    │    │ opentrace-    │  │
│  │  (调用 bpf)  │───▶│ server (HTTP) │  │
│  └─────────────┘    └───────────────┘  │
└──────────────────┬──────────────────────┘
                   │
┌──────────────────▼──────────────────────┐
│           opentrace-bpf                 │
└─────────────────────────────────────────┘
```

## 启动服务

```bash
sudo ./target/debug/opentrace-mcp
# 或
sudo cargo run --package opentrace-mcp
```

服务默认监听 `0.0.0.0:8080`。

## 健康检查

```bash
curl http://127.0.0.1:8080/mcp/healthz
```

## 工具

### skbdrop

追踪 skb drop 事件。

#### 参数

| 参数 | 类型 | 说明 |
|------|------|------|
| `any_host` | string | 匹配源地址或目的地址 |
| `src_host` | string | 匹配源地址 |
| `dst_host` | string | 匹配目的地址 |
| `any_port` | number | 匹配源端口或目的端口 |
| `src_port` | number | 匹配源端口 |
| `dst_port` | number | 匹配目的端口 |
| `ip_proto` | string | IP 协议（`tcp` / `udp` / `icmp` 或协议号 `6` / `17` / `1`） |
| `eth_proto` | string | 以太网协议（`ipv4` / `ipv6` 或协议号 `0x0800` / `0x86DD`） |

#### 返回值

调用后服务端等待 skb drop 事件并返回匹配项；若超时未捕获到事件则返回空结果。

### perf

CPU 性能剖析工具。

#### 参数

| 参数 | 类型 | 说明 |
|------|------|------|
| `pid` | number | 目标进程 PID，0 表示采样所有进程 |
| `cpu` | number | 目标 CPU，-1 表示所有 CPU |
| `duration` | number | 采样持续时间（秒） |

## 配置

### 命令行参数

| 参数 | 默认值 | 说明 |
|------|--------|------|
| `-p, --port` | `8080` | 监听端口 |
| `--bearer-token` | - | Bearer token 认证（可选） |
| `--tls-cert` | - | TLS 证书文件路径（可选，启用 HTTPS） |
| `--tls-key` | - | TLS 私钥文件路径（可选） |
| `--client-ca` | - | 客户端 CA 证书路径（可选，启用 mTLS） |

### 示例

```bash
# 基本启动
sudo ./target/debug/opentrace-mcp --port 9999

# 启用 Bearer token 认证
sudo ./target/debug/opentrace-mcp --bearer-token my-secret-token

# 启用 TLS
sudo ./target/debug/opentrace-mcp --tls-cert /path/to/cert.pem --tls-key /path/to/key.pem

# 启用 mTLS（双向认证）
sudo ./target/debug/opentrace-mcp --tls-cert cert.pem --tls-key key.pem --client-ca ca.pem
```

## 集成示例

### 与 AI 工具集成

MCP 服务可以与支持 MCP 协议的 AI 工具集成，实现自然语言驱动的网络诊断。

### API 调用示例

```bash
# 追踪 TCP 端口 80 的丢包
curl -X POST http://127.0.0.1:8080/mcp \
  -H "Content-Type: application/json" \
  -d '{"method": "tools/call", "params": {"name": "skbdrop", "arguments": {"dst_port": 80, "ip_proto": "tcp"}}}'
```
