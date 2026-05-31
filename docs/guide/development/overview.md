# 开发概览

## 架构

```
用户入口层                    核心能力层
┌─────────────┐              
│ opentrace-cli├──────────────┐
└─────────────┘              │
┌─────────────┐              ▼
│ opentrace-mcp├──────► opentrace-bpf
└─────────────┘         (Collectors/Exporters/Formatters)
```

- **opentrace-cli**: 命令行工具
- **opentrace-mcp**: MCP 服务（HTTP）
- **opentrace-bpf**: 核心库（eBPF 采集、导出、格式化）

## 目录结构

```
crates/
├── opentrace-bpf/src/
│   ├── bpf/            # BPF skeleton
│   ├── collectors/     # Collector 实现
│   ├── exporters/      # Exporter 实现
│   ├── formatter.rs    # Formatter trait
│   └── protocols/      # 协议解析器
│
├── opentrace-cli/src/
│   └── commands/       # 命令实现
│
└── opentrace-mcp/src/
    └── tools/          # MCP 工具
```

---

## 快速开发：创建新 Collector

### 步骤 1：定义 Event

```rust
// crates/opentrace-bpf/src/collectors/your_domain/event.rs

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
// crates/opentrace-bpf/src/collectors/your_domain/collector.rs

use crate::bpf::your_probe::{YourSkel, YourSkelBuilder};
use crate::collectors::macros::{define_collector, attach_tracepoint};
use crate::exporters::{Exporter, helper::load_and_dispatch};

define_collector!(YourCollector, YourSkel);

impl<'a> YourCollector<'a> {
    pub fn new(
        object: &'a mut MaybeUninit<OpenObject>,
        registry: &'a ProbeRegistry,
        mut exporter: impl Exporter<YourEvent> + 'a,
    ) -> Result<Self, EbpfError> {
        let skel = YourSkelBuilder::default().open(object)?.load()?;

        let perf_buffer = PerfBufferBuilder::new(&skel.maps.perf_events)
            .sample_cb(move |_cpu: i32, data: &[u8]| {
                load_and_dispatch::<YourEvent, _>(data, &mut exporter);
            })
            .build()?;

        Ok(Self { probe_registry: registry, skel, perf_buffer, _links: Vec::new() })
    }

    fn do_attach_probes(&mut self) -> Result<(), EbpfError> {
        attach_tracepoint!(self, "syscalls", tp_your_tracepoint);
        Ok(())
    }
}
```

### 步骤 3：实现 Formatter

```rust
// crates/opentrace-bpf/src/collectors/your_domain/formatter.rs

pub struct YourFormatter;

impl StreamFormatter<YourEvent> for YourFormatter {
    fn format<W: Write>(&self, w: &mut W, event: &YourEvent) -> io::Result<()> {
        writeln!(w, "[{}] pid={} tid={}", event.timestamp, event.pid, event.tid)
    }
}
```

### 步骤 4：注册模块

```rust
// crates/opentrace-bpf/src/collectors/your_domain/mod.rs
mod event;
mod collector;
mod formatter;

pub use event::YourEvent;
pub use collector::YourCollector;
pub use formatter::YourFormatter;
```

```rust
// crates/opentrace-bpf/src/collectors/mod.rs
mod your_domain;
pub use your_domain::{YourCollector, YourEvent, YourFormatter};
```

### 步骤 5：CLI 使用

```rust
// crates/opentrace-cli/src/commands/your_command.rs

pub fn run(registry: &mut ProbeRegistry, object: &mut CollectorObject) -> Result<()> {
    let formatter = YourFormatter;
    let exporter = DefaultStdoutExporter::new(formatter);
    let mut collector = YourCollector::new(object, registry, exporter)?;

    collector.attach_probe()?;
    loop {
        collector.poll(Duration::from_millis(100))?;
    }
}
```

---

## 核心 Trait

```rust
// Collector - 由宏自动实现
pub trait Collector {
    fn poll(&mut self, interval: Duration) -> Result<(), EbpfError>;
    fn attach_probe(&mut self) -> Result<(), EbpfError>;
}

// Exporter - 数据导出
pub trait Exporter<T> {
    fn dispatch(&mut self, event: T);
}

// StreamFormatter - 格式化输出
pub trait StreamFormatter<T> {
    fn format<W: Write>(&self, w: &mut W, args: &T) -> io::Result<()>;
}
```

## 探针宏

| 宏 | 用途 |
|---|---|
| `define_collector!(Name, Skel)` | 定义 Collector |
| `attach_tracepoint!(self, "category", tp_xxx)` | 挂载 tracepoint |
| `attach_kprobe!(self, kp_xxx, "func")` | 挂载 kprobe |

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

在 `new()` 中写入 BPF map：

```rust
skel.maps.config_map.update(
    &0u8.to_ne_bytes(),
    Into::<InnerConfig>::into(config).as_bytes(),
    MapFlags::ANY,
)?;
```
