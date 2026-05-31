# 协议扩展开发指南

本文档介绍如何为 OpenTrace 添加自定义应用层协议解析器。

## 概述

`SocketTraceCollector` 通过 `ProtoParser` trait 实现协议解析的可插拔设计。协议解析器将 eBPF 采集的原始字节解析为 `ParsedFrame`，由 Collector 自动完成请求-响应配对。

```
eBPF ──► InnerEvent ──► EventCacheStorage ──► ProtoParser.parse()
                                                │
                                                ▼
                                           ParsedFrame
                                                │
                                    ┌───────────┴───────────┐
                                    ▼                       ▼
                              request 缓存              response 配对
                                    │                       │
                                    └───────────┬───────────┘
                                                ▼
                                         Exporter 输出
```

---

## 核心 Trait

### `ProtoParser`

```rust
// crates/opentrace-bpf/src/protocols/mod.rs

pub trait ProtoParser {
    type Output: ParsedFrame;
    fn parse(&self, data: &[u8], size: usize, verbose: bool) -> Option<Self::Output>;
    fn hash_id(&self, data: &[u8], size: usize) -> u32;
}
```

| 方法 | 说明 |
|------|------|
| `Output` | 关联类型，必须实现 `ParsedFrame` |
| `parse()` | 将原始字节解析为 `ParsedFrame`，返回 `None` 表示无法识别该协议 |
| `hash_id()` | 生成请求-响应配对的标识符，相同请求和响应应返回相同值 |

### `ParsedFrame`

```rust
pub trait ParsedFrame {
    fn message_type(&self) -> MessageType;
    fn payload(&mut self) -> Option<Box<str>>;
    fn target(&mut self) -> Option<Box<str>>;
}

pub enum MessageType {
    Request,
    Response,
    Unknown,
}
```

| 方法 | 说明 |
|------|------|
| `message_type()` | 消息类型（请求/响应/未知） |
| `payload()` | 消息体，verbose 模式下显示 |
| `target()` | 请求目标（如 `GET /api`），简洁模式下显示 |

---

## 开发步骤

### 步骤 1：创建协议文件

```
crates/opentrace-bpf/src/protocols/
├── mod.rs
├── http.rs          # 已有
├── custom.rs        # 新增
└── ...
```

### 步骤 2：实现 `ParsedFrame`

```rust
// crates/opentrace-bpf/src/protocols/custom.rs

use super::{MessageType, ParsedFrame};

pub struct CustomFrame {
    pub message_type: MessageType,
    pub command: Option<Box<str>>,
    pub body: Option<Box<str>>,
}

impl ParsedFrame for CustomFrame {
    fn message_type(&self) -> MessageType {
        self.message_type
    }

    fn payload(&mut self) -> Option<Box<str>> {
        self.body.take()
    }

    fn target(&mut self) -> Option<Box<str>> {
        self.command.take()
    }
}
```

### 步骤 3：实现 `ProtoParser`

协议格式示例（简单二进制协议）：

```
字节偏移    内容
[0..2]     魔数 0xCSTP
[2]        消息类型: 0x01=请求, 0x02=响应
[3]        命令长度 N
[4..4+N]   命令字符串
[4+N..]    消息体
```

```rust
// crates/opentrace-bpf/src/protocols/custom.rs（续）

use super::ProtoParser;

const MAGIC: [u8; 2] = [0xC5, 0x50];
const HEADER_LEN: usize = 4;

#[derive(Default)]
pub struct CustomParser;

impl ProtoParser for CustomParser {
    type Output = CustomFrame;

    fn parse(&self, data: &[u8], size: usize, verbose: bool) -> Option<Self::Output> {
        let data = &data[..data.len().min(size)];

        // 1. 魔数校验
        if data.len() < HEADER_LEN || data[..2] != MAGIC {
            return None;
        }

        // 2. 解析头部
        let message_type = match data[2] {
            0x01 => MessageType::Request,
            0x02 => MessageType::Response,
            _ => return None,
        };
        let cmd_len = data[3] as usize;

        // 3. 解析命令
        let cmd_end = HEADER_LEN + cmd_len;
        let command = std::str::from_utf8(data.get(HEADER_LEN..cmd_end)?)
            .ok()
            .map(Into::into);

        // 4. 解析消息体（仅 verbose）
        let body = if verbose {
            std::str::from_utf8(data.get(cmd_end..)?)
                .ok()
                .filter(|s| !s.is_empty())
                .map(Into::into)
        } else {
            None
        };

        Some(CustomFrame {
            message_type,
            command,
            body,
        })
    }

    fn hash_id(&self, data: &[u8], size: usize) -> u32 {
        let data = &data[..data.len().min(size)];
        if data.len() < HEADER_LEN {
            return 0;
        }
        // 使用命令内容做 hash
        let cmd_len = data[3] as usize;
        let cmd = data.get(HEADER_LEN..HEADER_LEN + cmd_len).unwrap_or(&[]);
        let mut hash: u32 = 0;
        for &b in cmd {
            hash = hash.wrapping_mul(31).wrapping_add(b as u32);
        }
        hash
    }
}
```

### 步骤 4：注册模块

```rust
// crates/opentrace-bpf/src/protocols/mod.rs

pub mod custom;    // 新增

pub mod app_protos {
    pub use super::MessageType;
    pub use super::http::{HttpDirection, HttpFrame, HttpMethod, HttpParser};
    pub use super::custom::{CustomFrame, CustomParser};    // 新增
}
```

`lib.rs` 中 `protocol` 模块已导出 `ProtoParser`、`ParsedFrame`、`MessageType`，无需修改。

### 步骤 5：使用

```rust
use opentrace_bpf::collector::net::{SocketTraceCollector, SocketDefaultFormatter};
use opentrace_bpf::exporter::DefaultStdoutExporter;
use opentrace_bpf::protocol::appproto::CustomParser;

let formatter = SocketDefaultFormatter::new(false);
let exporter = DefaultStdoutExporter::new(formatter);
let parser = CustomParser;

let mut collector = SocketTraceCollector::new(
    object, registry, config, exporter, parser,
)?;

collector.attach_probe()?;
loop {
    let _ = collector.poll(Duration::from_millis(100));
}
```

---

## Trait 方法详解

### `parse()`

| 参数 | 说明 |
|------|------|
| `data` | 原始字节缓冲区（最大 1024 字节） |
| `size` | 实际数据长度 |
| `verbose` | `false` 时只需解析 `target`；`true` 时解析完整数据 |

- 返回 `None` 表示数据不是该协议，Collector 会跳过
- 先做魔数校验快速排除
- 解析失败返回 `None`，不要 panic

### `hash_id()`

用于请求-响应配对。Collector 根据 `message_type()` 和连接方向判断请求/响应，用相同 `hash_id` 配对。

实现建议：
- 有 request ID 的协议直接使用
- 无明确 ID 的协议对命令/URL 做 hash
- 返回 `0` 会导致每个请求都生成新 Event

---

## 调试

在 `parse()` 开头打印原始数据：

```rust
eprintln!("[CustomParser] {} bytes: {:02x?}", data.len(), data);
```

单元测试：

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_request() {
        let mut data = vec![0xC5, 0x50, 0x01, 0x03]; // 魔数 + 请求 + cmd_len=3
        data.extend_from_slice(b"GET");
        data.extend_from_slice(b"body");

        let parser = CustomParser;
        let frame = parser.parse(&data, data.len(), false).unwrap();
        assert_eq!(frame.message_type(), MessageType::Request);
        assert_eq!(frame.target().unwrap().as_ref(), "GET");
    }

    #[test]
    fn test_not_my_protocol() {
        let data = b"HTTP/1.1 200 OK";
        let parser = CustomParser;
        assert!(parser.parse(data, data.len(), false).is_none());
    }
}
```

```bash
cargo test -p opentrace-bpf -- custom
```

---

## 参考

内置 `HttpParser`（`protocols/http.rs`）包含完整的 HTTP/1.x + HTTP/2 解析实现。
