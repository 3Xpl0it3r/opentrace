# CLI 命令

OpenTrace CLI 是主要的命令行工具，用于追踪内核网络事件。

## 基本用法

```bash
sudo opentrace-cli [全局选项] <命令> [子命令] [选项]
```

### 全局选项

| 参数 | 说明 |
|------|------|
| `--custom-btf-path <PATH>` | 自定义 BTF 路径（用于不支持 BTF 的内核） |

## 命令

### trace

追踪内核事件。

#### trace skbdrop

追踪 skb drop 事件：

```bash
sudo opentrace-cli trace skbdrop
```

##### 过滤表达式

支持以下过滤表达式：

| 表达式 | 说明 | 示例 |
|--------|------|------|
| `tcp` | TCP 协议 | `-f "tcp"` |
| `udp` | UDP 协议 | `-f "udp"` |
| `icmp` | ICMP 协议 | `-f "icmp"` |
| `host` | 源或目的地址 | `-f "host 10.0.0.1"` |
| `src host` | 源地址 | `-f "src host 10.0.0.1"` |
| `dst host` | 目的地址 | `-f "dst host 10.0.0.1"` |
| `port` | 源或目的端口 | `-f "port 80"` |
| `src port` | 源端口 | `-f "src port 12345"` |
| `dst port` | 目的端口 | `-f "dst port 443"` |

组合示例：

```bash
# TCP 端口 80
sudo opentrace-cli trace skbdrop -f "tcp port 80"

# 特定主机和端口
sudo opentrace-cli trace skbdrop -f "host 10.0.0.1 and tcp and port 443"

# 源地址和目的端口
sudo opentrace-cli trace skbdrop -f "src host 10.0.0.1 and dst port 443"

# UDP DNS
sudo opentrace-cli trace skbdrop -f "udp port 53"
```

##### 命令行参数

| 参数 | 说明 |
|------|------|
| `-f, --filter <EXPR>` | 过滤表达式 |
| `-i, --iface <IFACE>` | 指定网络接口 |
| `-p, --pid <PID>` | 按进程 ID 过滤 |
| `--pname <NAME>` | 按进程名过滤 |
| `--container-id <ID>` | 按容器 ID 过滤 |
| `--container-name <NAME>` | 按容器名过滤 |
| `--pod <POD>` | 按 Kubernetes Pod 名称过滤 |
| `-6, --v6` | 启用 IPv6 相关参数 |

### perf

CPU 性能剖析。

#### perf profile

CPU 性能采样：

```bash
sudo opentrace-cli perf profile
```

##### 命令行参数

| 参数 | 说明 |
|------|------|
| `-p, --pid <PID>` | 按进程 ID 过滤（不指定则采样所有进程） |
| `--tid <TID>` | 按线程 ID 过滤（需要同时指定 pid） |
| `-c, --cpu <CPU>` | 指定 CPU（-1 表示所有 CPU） |
| `-g, --group <GROUP>` | 指定 cgroup ID（-1 表示所有） |
| `--language <LANG>` | 符号解析扩展（支持 `java`） |

##### 示例

```bash
# 采样所有进程
sudo opentrace-cli perf profile

# 采样指定进程
sudo opentrace-cli perf profile --pid 1234

# 采样指定进程的特定线程
sudo opentrace-cli perf profile --pid 1234 --tid 5678

# 在指定 CPU 上采样
sudo opentrace-cli perf profile --cpu 0

# Java 符号解析
sudo opentrace-cli perf profile --pid 1234 --language java
```

### watch

监控网络连接。

#### watch http

监控 HTTP 流量：

```bash
sudo opentrace-cli watch http
```

##### 命令行参数

| 参数 | 说明 |
|------|------|
| `-p, --pid <PID>` | 按进程 ID 过滤 |
| `-v, --verbose` | 详细输出 |

##### 示例

```bash
# 监控所有 HTTP 流量
sudo opentrace-cli watch http

# 监控指定进程的 HTTP 流量
sudo opentrace-cli watch http --pid 1234

# 详细模式
sudo opentrace-cli watch http --pid 1234 --verbose
```

## 输出格式

### 默认输出

```
   PID   PPID COMMAND          FILE                ARGS
--------------------------------------------------------------------------------
  1234   1000 bash             /bin/bash           bash
  1235   1234 ls               /bin/ls             ls -la /tmp
```

### JSON 输出

使用 `--json` 参数输出 JSON 格式：

```bash
sudo opentrace-cli trace skbdrop --json
```

### 详细输出

使用 `--verbose` 参数输出详细信息：

```bash
sudo opentrace-cli trace skbdrop --verbose
```
