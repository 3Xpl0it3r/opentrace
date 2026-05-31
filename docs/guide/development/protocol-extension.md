# 协议扩展

为 `SocketTraceCollector` 添加自定义协议解析器。

## 流程

```
eBPF ─► Event ─► ProtoParser.parse() ─► ParsedFrame ─► request/response 配对 ─► Exporter
```

## Trait

```rust
pub trait ProtoParser {
    type Output: ParsedFrame;
    fn parse(&self, data: &[u8], size: usize, verbose: bool) -> Option<Self::Output>;
    fn hash_id(&self, data: &[u8], size: usize) -> u32;  // 请求-响应配对用
}

pub trait ParsedFrame {
    fn message_type(&self) -> MessageType;  // Request / Response / Unknown
    fn payload(&mut self) -> Option<Box<str>>;  // verbose 时显示
    fn target(&mut self) -> Option<Box<str>>;   // 简洁模式显示（如 "GET /api"）
}
```

---

## 开发步骤

### 1. 创建文件

```
crates/opentrace-bpf/src/protocols/custom.rs
```

### 2. 实现解析器

```rust
// crates/opentrace-bpf/src/protocols/custom.rs

use super::{MessageType, ParsedFrame, ProtoParser};

pub struct CustomFrame { /* ... */ }

impl ParsedFrame for CustomFrame {
    fn message_type(&self) -> MessageType { /* ... */ }
    fn payload(&mut self) -> Option<Box<str>> { /* ... */ }
    fn target(&mut self) -> Option<Box<str>> { /* ... */ }
}

#[derive(Default)]
pub struct CustomParser;

impl ProtoParser for CustomParser {
    type Output = CustomFrame;

    fn parse(&self, data: &[u8], size: usize, verbose: bool) -> Option<Self::Output> {
        let data = &data[..data.len().min(size)];
        if data.len() < 4 || data[..2] != [0xC5, 0x50] { return None; }  // 魔数校验

        let message_type = match data[2] { /* 0x01=Request, 0x02=Response, _ => return None */ };
        let command = /* 解析命令 */;
        let body = if verbose { /* 解析消息体 */ } else { None };

        Some(CustomFrame { message_type, command, body })
    }

    fn hash_id(&self, data: &[u8], size: usize) -> u32 {
        // 对命令做 hash，用于请求-响应配对
        0 // 替换为实际实现
    }
}
```

### 3. 注册

```rust
// crates/opentrace-bpf/src/protocols/mod.rs
pub mod custom;

pub mod app_protos {
    pub use super::custom::{CustomFrame, CustomParser};
    // ... 已有协议
}
```

### 4. 使用

```rust
use opentrace_bpf::protocol::appproto::CustomParser;

let parser = CustomParser;
let mut collector = SocketTraceCollector::new(object, registry, config, exporter, parser)?;
collector.attach_probe()?;
loop { collector.poll(Duration::from_millis(100))?; }
```

---

## 要点

- `parse()` 返回 `None` 表示不是该协议，Collector 会跳过
- `hash_id()` 用于配对请求和响应，有 request ID 直接用，没有就对命令做 hash
- 先做魔数校验快速排除
- 参考 `protocols/http.rs` 的实现
