# MCP 服务

OpenTrace 支持通过 MCP (Model Context Protocol) 服务调用，便于集成到 AI 工具链。

## 启动服务

```bash
sudo ./target/debug/opentrace-server
# 或
sudo cargo run --package opentrace-server
```

服务默认监听 `0.0.0.0:9999`。

## 健康检查

```bash
curl http://127.0.0.1:9999/healthz
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

## 配置

### 环境变量

| 变量 | 默认值 | 说明 |
|------|--------|------|
| `OPENTRACE_HOST` | `0.0.0.0` | 监听地址 |
| `OPENTRACE_PORT` | `9999` | 监听端口 |

## 集成示例

### 与 AI 工具集成

MCP 服务可以与支持 MCP 协议的 AI 工具集成，实现自然语言驱动的网络诊断。

### API 调用示例

```bash
# 追踪 TCP 端口 80 的丢包
curl -X POST http://127.0.0.1:9999/tools/skbdrop \
  -H "Content-Type: application/json" \
  -d '{"dst_port": 80, "ip_proto": "tcp"}'
```
