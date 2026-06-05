# 开发概览

## 架构

```
用户入口层:
┌──────────────┐    ┌─────────────┐    ┌──────────────┐
│ opentrace-cli │    │ opentrace   │    │ opentrace    │
│ (命令行)      │    │ -mcp (MCP) │    │ -agent (REST)│
└──────┬───────┘    └──────┬──────┘    └──────┬───────┘
       │                   │                   │
       ▼                   ▼                   ▼
┌──────────────────────────────────────────────────┐
│              opentrace-kit                        │
│      通用 HTTP Server (axum/TLS/Auth)             │
└──────────────────────────────────────────────────┘
       │                   │
       ▼                   ▼
┌──────────────────────────────────────────────────┐
│              opentrace-bpf                        │
│  Collector / Sink / Formatter / Protocol         │
│  Symbolizer / ProbeRegistry                      │
└──────────────────────────────────────────────────┘
```

- **opentrace-cli**: 命令行工具，直接调用 opentrace-bpf
- **opentrace-mcp**: MCP 服务，通过 opentrace-kit 提供 HTTP
- **opentrace-agent**: Agent 服务，带 Prometheus 指标 + REST API
- **opentrace-kit**: 通用 HTTP 服务器框架
- **opentrace-bpf**: 核心库（eBPF 采集、数据导出、格式化、协议解析、符号解析）

## 目录结构

```
crates/
├── opentrace-bpf/src/
│   ├── bpf/            # BPF C 程序 + libbpf 源码
│   ├── collector/      # Collector trait + 实现 (net/cpu)
│   ├── sink/           # EventSink trait (channel/stream_writer)
│   ├── formatter.rs    # Formatter trait
│   ├── protocol/       # 协议解析器 (ether/ip/http)
│   ├── probe/          # ProbeRegistry 探针注册表
│   ├── symbolizer/     # 符号解析 (内核/用户态/Go/Java)
│   ├── types/          # repr(C) 数据结构
│   ├── utils/          # 工具函数
│   ├── env.rs          # BTF 检测 / memlock
│   └── testing/        # Mock (feature = "testing")
│
├── opentrace-cli/src/
│   └── commands/       # 命令实现 (trace/perf/watch)
│
├── opentrace-mcp/src/
│   └── tools/          # MCP 工具 (skbdrop/perf)
│
├── opentrace-agent/src/
│   ├── agent.rs        # OpentraceAgent 主结构体
│   ├── manager/        # Exporter 生命周期管理
│   ├── exporter/       # Prometheus 指标导出
│   ├── api/            # REST API (axum)
│   └── errors.rs       # AgntError
│
└── opentrace-kit/src/
    └── server/         # 通用 HTTP Server (axum)
        ├── server.rs
        ├── config.rs
        ├── authentication.rs
        └── errors.rs
```

---

## 快速开发：创建新 Collector

### 步骤 1：定义 Event

```rust
// crates/opentrace-bpf/src/collector/your_domain/event.rs

#[derive(Clone, Copy)]
#[repr(C)]
pub struct YourEvent {
    pub pid: u32,
    pub tid: u32,
    pub comm: [u8; 16],
    pub timestamp: u64,
}
```

### 步骤 2：创建 Collector

```rust
// crates/opentrace-bpf/src/collector/your_domain/collector.rs

use crate::bpf::your_probe::{YourSkel, YourSkelBuilder};
use crate::collector::macros::{define_collector, attach_tracepoint};
use crate::sink::{EventSink, helper::load_and_dispatch};

define_collector!(YourCollector, YourSkel);

impl<'a> YourCollector<'a> {
    pub fn new(
        object: &'a mut MaybeUninit<OpenObject>,
        mut exporter: impl EventSink<YourEvent> + 'a,
    ) -> Result<Self, EbpfError> {
        let skel = YourSkelBuilder::default().open(object)?.load()?;

        let perf_buffer = PerfBufferBuilder::new(&skel.maps.perf_events)
            .sample_cb(move |_cpu: i32, data: &[u8]| {
                load_and_dispatch::<YourEvent, _>(data, &mut exporter);
            })
            .build()?;

        Ok(Self { skel, perf_buffer, _links: Vec::new() })
    }

    fn do_attach_probes(&mut self, probe_registry: &ProbeRegistry) -> Result<(), EbpfError> {
        attach_tracepoint!(self, probe_registry, "syscalls", tp_your_tracepoint);
        Ok(())
    }
}
```

### 步骤 3：实现 Formatter

```rust
// crates/opentrace-bpf/src/collector/your_domain/formatter.rs

pub struct YourFormatter;

impl StreamFormatter<YourEvent> for YourFormatter {
    fn format<W: Write>(&self, w: &mut W, event: &YourEvent) -> io::Result<()> {
        writeln!(w, "[{}] pid={} tid={}", event.timestamp, event.pid, event.tid)
    }
}
```

### 步骤 4：注册模块

```rust
// crates/opentrace-bpf/src/collector/your_domain/mod.rs
mod event;
mod collector;
mod formatter;

pub use event::YourEvent;
pub use collector::YourCollector;
pub use formatter::YourFormatter;
```

```rust
// crates/opentrace-bpf/src/collector/mod.rs
mod your_domain;
pub use your_domain::{YourCollector, YourEvent, YourFormatter};
```

### 步骤 5：CLI 使用

```rust
// crates/opentrace-cli/src/commands/your_command.rs

pub fn run(registry: &mut ProbeRegistry, object: &mut CollectorObject) -> Result<()> {
    let formatter = YourFormatter;
    let exporter = StreamWriterSink::new(formatter);
    let mut collector = YourCollector::new(object, exporter)?;

    collector.attach_probe(registry)?;
    loop {
        collector.poll(Duration::from_millis(100))?;
    }
}
```

---

## 核心 Trait

```rust
// Collector - 由 define_collector! 宏自动实现
pub trait Collector: Send {
    fn poll(&mut self, interval: Duration) -> Result<(), EbpfError>;
    fn attach_probe(&mut self, probe_registry: &ProbeRegistry) -> Result<(), EbpfError>;
}

// EventSink - 数据导出
pub trait EventSink<T> {
    fn load(&self, data: &[u8]) -> T;    // 内核→用户态反序列化
    fn dispatch(&mut self, event: T);     // 用户态→外部生态
}

// StreamFormatter - 格式化输出
pub trait StreamFormatter<T> {
    fn format<W: Write>(&self, w: &mut W, args: &T) -> io::Result<()>;
}
```

## 探针宏

| 宏 | 用途 |
|---|---|
| `define_collector!(Name, Skel)` | 定义 Collector，生成带 `probe_registry: &ProbeRegistry` 参数的 `attach_probe` |
| `attach_tracepoint!(self, registry, "category", tp_xxx)` | 挂载 tracepoint（需传入 `ProbeRegistry`） |
| `attach_kprobe!(self, registry, kp_xxx, "func")` | 挂载 kprobe（需传入 `ProbeRegistry`） |
| `attach_kretprobe!(self, registry, kp_xxx, "func")` | 挂载 kretprobe（需传入 `ProbeRegistry`） |
| `attach_perf_event!(self, prog, pfd)` | 挂载 perf event（无需 registry） |

---

## 有配置的 Collector

如果需要过滤配置，添加 Config：

```rust
// config.rs
#[derive(Default)]
pub struct Config {
    pub pid: u32,  // 0 表示不过滤
}

#[repr(C)]
pub(crate) struct InnerConfig {
    pid: u32,
    _pad: [u8; 4],
}

impl From<Config> for InnerConfig {
    fn from(c: Config) -> Self {
        InnerConfig { pid: c.pid, _pad: [0; 4] }
    }
}
```

在 `new()` 中写入 BPF map（注意：`new()` 不再接收 `registry` 参数，`ProbeRegistry` 仅在 `attach_probe` 时传入）：

```rust
skel.maps.config_map.update(
    &0u8.to_ne_bytes(),
    Into::<InnerConfig>::into(config).as_bytes(),
    MapFlags::ANY,
)?;
```
