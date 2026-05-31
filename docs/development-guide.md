# OpenTrace 二次开发指南

## 架构概览

OpenTrace 采用三层架构设计：

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

### 核心组件

| 组件 | 作用 | 位置 |
|------|------|------|
| **Collector** | 用户态 eBPF 程序，负责挂载探针、采集内核数据 | `crates/opentrace-bpf/src/collectors/` |
| **Exporter** | 数据导出器，将采集的数据发送到目标（终端/ES/Kafka等） | `crates/opentrace-bpf/src/exporters/` |
| **Formatter** | 数据格式化器，将 Event 格式化为可读字符串 | `crates/opentrace-bpf/src/formatter.rs` |

---

## 开发流程

### 步骤 1: 定义 Event 结构体

Event 是从内核态传递到用户态的数据结构，需要 `#[repr(C)]` 确保内存布局一致。

```rust
// crates/opentrace-bpf/src/collectors/your_domain/your_event.rs

use serde::{Serialize, Deserialize};

#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct YourEvent {
    pub pid: u32,
    pub tid: u32,
    pub comm: [u8; 16],
    pub timestamp: u64,
    // ... 其他字段
}

// 可选：实现序列化用于 JSON 输出
impl Serialize for YourEvent {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        // 自定义序列化逻辑
    }
}
```

### 步骤 2: 定义 Config 结构体

Config 用于向 eBPF 程序传递配置参数。

```rust
// crates/opentrace-bpf/src/collectors/your_domain/your_collector.rs

use std::mem;

/// 用户态配置
#[derive(Default, Debug)]
pub struct Config {
    pub pid: u32,
    pub custom_btf_path: Option<String>,
    // ... 其他配置项
}

/// 传递给内核态的配置（与 BPF 程序中的结构体对应）
#[repr(C)]
struct InnerConfig {
    pid: u32,
    _pad: [u8; 4],
}

impl InnerConfig {
    fn as_bytes(&self) -> &[u8] {
        let ptr = self as *const Self as *const u8;
        let len = mem::size_of_val(self);
        unsafe { std::slice::from_raw_parts(ptr, len) }
    }
}

impl From<Config> for InnerConfig {
    fn from(config: Config) -> Self {
        InnerConfig {
            pid: config.pid,
            _pad: [0; 4],
        }
    }
}
```

### 步骤 3: 创建 Collector（使用宏）

使用 `define_collector!` 宏快速创建 Collector 结构体并自动实现 `Collector` trait。

```rust
// crates/opentrace-bpf/src/collectors/your_domain/your_collector.rs

use std::mem::MaybeUninit;
use libbpf_rs::skel::{OpenSkel, SkelBuilder};
use libbpf_rs::{MapCore, MapFlags, OpenObject, PerfBufferBuilder};

use crate::bpf::your_probe::{YourSkel, YourSkelBuilder};
use crate::collectors::macros::{define_collector, attach_kprobe, attach_tracepoint};
use crate::exporters::{Exporter, helper::load_and_dispatch};
use crate::EbpfError;
use crate::ProbeRegistry;

const CONFIG_KEY: u8 = 0;

// 使用宏定义 Collector 结构体并实现 Collector trait
define_collector!(YourCollector, YourSkel);

impl<'a> YourCollector<'a> {
    pub fn new(
        object: &'a mut MaybeUninit<OpenObject>,
        registry: &'a ProbeRegistry,
        config: Config,
        mut exporter: impl Exporter<YourEvent> + 'a,
    ) -> Result<Self, EbpfError> {
        // 1. 加载 BPF skeleton
        let skel = match config.custom_btf_path {
            Some(ref path) => crate::skeleton::with_custom_btf_open_opts(path, |open_opts| {
                Ok(YourSkelBuilder::default()
                    .open_opts(open_opts, object)?
                    .load()?)
            })?,
            None => YourSkelBuilder::default().open(object)?.load()?,
        };

        // 2. 写入配置到 BPF map
        skel.maps.config_map.update(
            &CONFIG_KEY.to_ne_bytes(),
            Into::<InnerConfig>::into(config).as_bytes(),
            MapFlags::ANY,
        ).map_err(EbpfError::Libbpf)?;

        // 3. 创建 PerfBuffer，设置回调函数
        let perf_buffer = PerfBufferBuilder::new(&skel.maps.perf_events)
            .sample_cb(move |_cpu: i32, data: &[u8]| {
                // 固定写法：加载数据并分发给 exporter
                load_and_dispatch::<YourEvent, _>(data, &mut exporter);
            })
            .build()?;

        Ok(Self {
            probe_registry: registry,
            skel,
            perf_buffer,
            _links: Vec::new(),
        })
    }

    /// 实现探针挂载逻辑（由宏中的 attach_probe 调用）
    fn do_attach_probes(&mut self) -> Result<(), EbpfError> {
        // 使用宏挂载探针
        attach_tracepoint!(self, "syscalls", tp_your_tracepoint);
        attach_kprobe!(self, kp_your_function, "your_kernel_function");
        // ... 更多探针

        Ok(())
    }
}
```

### 步骤 4: 实现 Formatter

实现 `StreamFormatter` trait 来格式化 Event 数据。

```rust
// crates/opentrace-bpf/src/collectors/your_domain/your_formatter.rs

use std::io::{self, Write};
use crate::formatter::StreamFormatter;
use super::your_collector::YourEvent;

pub struct YourFormatter {
    verbose: bool,
}

impl YourFormatter {
    pub fn new(verbose: bool) -> Self {
        Self { verbose }
    }
}

impl StreamFormatter<YourEvent> for YourFormatter {
    fn format<W: Write>(&self, w: &mut W, event: &YourEvent) -> io::Result<()> {
        if self.verbose {
            writeln!(w, "PID: {}", event.pid)?;
            writeln!(w, "TID: {}", event.tid)?;
            writeln!(w, "Timestamp: {}", event.timestamp)?;
            // ... 更多详细输出
        } else {
            writeln!(w, "[{}] pid={} tid={}", event.timestamp, event.pid, event.tid)?;
        }
        Ok(())
    }
}
```

### 步骤 5: 注册到 lib.rs

在 `crates/opentrace-bpf/src/lib.rs` 中导出你的模块：

```rust
// crates/opentrace-bpf/src/lib.rs

pub mod collector {
    pub use crate::collectors::Collector;
    
    // 已有的模块
    pub mod net { /* ... */ }
    pub mod cpu { /* ... */ }
    
    // 添加你的模块
    pub mod your_domain {
        pub use crate::collectors::your_domain::{
            YourCollector, YourConfig, YourEvent, YourFormatter,
        };
    }
}
```

### 步骤 6: 在 CLI 中使用

在 `crates/opentrace-cli/` 中添加命令：

```rust
// crates/opentrace-cli/src/commands/your_command.rs

use std::time::Duration;
use opentrace_bpf::ProbeRegistry;
use opentrace_bpf::collector::Collector;
use opentrace_bpf::collector::your_domain::{YourCollector, YourFormatter};
use opentrace_bpf::exporter::DefaultStdoutExporter;

pub fn run(
    registry: &mut ProbeRegistry,
    object: &mut opentrace_bpf::CollectorObject,
    config: YourConfig,
) -> Result<(), Box<dyn std::error::Error>> {
    // 1. 创建 Formatter
    let formatter = YourFormatter::new(true);
    
    // 2. 创建 Exporter（使用默认的 Stdout 输出）
    let exporter = DefaultStdoutExporter::new(formatter);
    
    // 3. 创建 Collector
    let mut collector = YourCollector::new(
        object,
        registry,
        config,
        exporter,
    )?;
    
    // 4. 挂载探针
    collector.attach_probe()?;
    
    // 5. 轮询事件
    loop {
        let _ = collector.poll(Duration::from_millis(100));
    }
}
```

---

## 核心 Trait 详解

### Collector Trait

```rust
pub trait Collector {
    /// 轮询 PerfBuffer 获取事件
    fn poll(&mut self, interval: Duration) -> Result<(), EbpfError>;
    
    /// 挂载 eBPF 探针
    fn attach_probe(&mut self) -> Result<(), EbpfError>;
}
```

使用 `define_collector!` 宏会自动实现此 trait，你只需实现 `do_attach_probes()` 方法。

### Exporter Trait

```rust
pub trait Exporter<T> {
    /// 从字节数据加载 Event（默认实现，通常不需要修改）
    fn load(&self, data: &[u8]) -> T {
        unsafe { std::ptr::read(data.as_ptr() as *const T) }
    }
    
    /// 分发事件到目标
    fn dispatch(&mut self, event: T);
}
```

内置 Exporter：
- `DefaultStdoutExporter` - 输出到终端
- `SimpleBoundChannelExporter` - 通过 channel 发送（有界）
- `SimpleUnboundChannelExporter` - 通过 channel 发送（无界）

### StreamFormatter Trait

```rust
pub trait StreamFormatter<T> {
    fn format<W: Write>(&self, w: &mut W, args: &T) -> io::Result<()>;
}
```

---

## 探针挂载宏

| 宏 | 用途 | 示例 |
|---|---|---|
| `define_collector!` | 定义 Collector 结构体 | `define_collector!(MyCollector, MySkel);` |
| `attach_tracepoint!` | 挂载单个 tracepoint | `attach_tracepoint!(self, "syscalls", tp_sys_enter_read);` |
| `attach_multiple_tracepoints!` | 批量挂载同类 tracepoint | `attach_multiple_tracepoints!(self, "syscalls", tp_sys_enter, ["accept", "accept4"]);` |
| `attach_kprobe!` | 挂载 kprobe | `attach_kprobe!(self, kp_tcp_connect, "tcp_connect");` |
| `attach_kretprobe!` | 挂载 kretprobe | `attach_kretprobe!(self, kret_tcp_connect, "tcp_connect");` |
| `attach_perf_event!` | 挂载 perf event | `attach_perf_event!(self, perf_profile_samples, pfd);` |

---

## 完整示例：创建 HTTP 请求追踪 Collector

### 文件结构

```
crates/opentrace-bpf/src/collectors/
├── mod.rs
├── macros.rs
└── http/
    ├── mod.rs
    ├── event.rs
    ├── config.rs
    ├── collector.rs
    └── formatter.rs
```

### 1. event.rs

```rust
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct HttpEvent {
    pub pid: u32,
    pub tid: u32,
    pub method: u8,       // 0=GET, 1=POST, ...
    pub status: u16,
    pub latency_us: u64,
    pub path: [u8; 128],
}
```

### 2. collector.rs

```rust
use std::mem::MaybeUninit;
use libbpf_rs::skel::{OpenSkel, SkelBuilder};
use libbpf_rs::{MapFlags, OpenObject, PerfBufferBuilder};

use crate::bpf::http_trace::{HttpTraceSkel, HttpTraceSkelBuilder};
use crate::collectors::macros::define_collector;
use crate::exporters::{Exporter, helper::load_and_dispatch};
use crate::{EbpfError, ProbeRegistry};

use super::event::HttpEvent;
use super::config::{Config, InnerConfig};

const CONFIG_KEY: u8 = 0;

define_collector!(HttpCollector, HttpTraceSkel);

impl<'a> HttpCollector<'a> {
    pub fn new(
        object: &'a mut MaybeUninit<OpenObject>,
        registry: &'a ProbeRegistry,
        config: Config,
        mut exporter: impl Exporter<HttpEvent> + 'a,
    ) -> Result<Self, EbpfError> {
        let skel = HttpTraceSkelBuilder::default()
            .open(object)?
            .load()?;

        skel.maps.config_map.update(
            &CONFIG_KEY.to_ne_bytes(),
            Into::<InnerConfig>::into(config).as_bytes(),
            MapFlags::ANY,
        ).map_err(EbpfError::Libbpf)?;

        let perf_buffer = PerfBufferBuilder::new(&skel.maps.perf_events)
            .sample_cb(move |_cpu: i32, data: &[u8]| {
                load_and_dispatch::<HttpEvent, _>(data, &mut exporter);
            })
            .build()?;

        Ok(Self {
            probe_registry: registry,
            skel,
            perf_buffer,
            _links: Vec::new(),
        })
    }

    fn do_attach_probes(&mut self) -> Result<(), EbpfError> {
        // 挂载 HTTP 相关探针
        Ok(())
    }
}
```

### 3. formatter.rs

```rust
use std::io::{self, Write};
use crate::formatter::StreamFormatter;
use super::event::HttpEvent;

pub struct HttpFormatter;

impl HttpFormatter {
    pub fn new() -> Self { Self }
}

impl StreamFormatter<HttpEvent> for HttpFormatter {
    fn format<W: Write>(&self, w: &mut W, event: &HttpEvent) -> io::Result<()> {
        let method = match event.method {
            0 => "GET",
            1 => "POST",
            _ => "UNKNOWN",
        };
        writeln!(w, "[{}] {} {} status={} latency={}us", 
            event.pid, method, 
            String::from_utf8_lossy(&event.path),
            event.status, event.latency_us)
    }
}
```

### 4. CLI 使用

```rust
use opentrace_bpf::collector::http::{HttpCollector, HttpFormatter, HttpConfig};
use opentrace_bpf::exporter::DefaultStdoutExporter;
use opentrace_bpf::collector::Collector;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let registry = opentrace_bpf::ProbeRegistry::new()?;
    let mut object = opentrace_bpf::open_object_storage();
    
    let formatter = HttpFormatter::new();
    let exporter = DefaultStdoutExporter::new(formatter);
    let config = HttpConfig::default();
    
    let mut collector = HttpCollector::new(&mut object, &registry, config, exporter)?;
    collector.attach_probe()?;
    
    loop {
        collector.poll(std::time::Duration::from_millis(100))?;
    }
}
```

---

## 自定义 Exporter

如果需要将数据发送到 Elasticsearch、Kafka 等，可以实现自定义 Exporter：

```rust
use opentrace_bpf::exporter::Exporter;
use opentrace_bpf::collector::your_domain::YourEvent;

pub struct ElasticsearchExporter {
    client: reqwest::Client,
    index: String,
}

impl ElasticsearchExporter {
    pub fn new(index: String) -> Self {
        Self {
            client: reqwest::Client::new(),
            index,
        }
    }
}

impl Exporter<YourEvent> for ElasticsearchExporter {
    fn dispatch(&mut self, event: YourEvent) {
        let json = serde_json::to_string(&event).unwrap();
        let url = format!("http://localhost:9200/{}/_doc", self.index);
        let _ = self.client.post(&url)
            .body(json)
            .header("Content-Type", "application/json")
            .send();
    }
}
```

---

## 目录结构参考

```
crates/
├── opentrace-bpf/           # 核心库
│   ├── src/
│   │   ├── bpf/            # BPF skeleton 生成
│   │   ├── collectors/     # Collector 实现
│   │   │   ├── mod.rs
│   │   │   ├── macros.rs
│   │   │   ├── net/        # 网络相关
│   │   │   └── cpu/        # CPU 相关
│   │   ├── exporters/      # Exporter 实现
│   │   ├── formatter.rs    # Formatter trait
│   │   ├── types/          # 公共类型
│   │   └── lib.rs
│   └── build.rs            # BPF 编译脚本
│
├── opentrace-cli/          # 命令行工具
│   └── src/
│       ├── commands/       # 命令实现
│       └── bin/
│
└── opentrace-server/       # 服务端（可选）
```

---

## 常见问题

### Q: 如何获取 BPF skeleton？

BPF skeleton 由 `build.rs` 自动生成，对应 `crates/opentrace-bpf/src/bpf/` 目录下的模块。

### Q: Event 结构体字段顺序重要吗？

是的，`#[repr(C)]` 要求字段顺序与内核态 BPF 程序中的结构体完全一致。

### Q: 如何调试 BPF 程序？

使用 `cat /sys/kernel/debug/tracing/trace_pipe` 查看内核日志，或在 BPF 程序中使用 `bpf_printk()`。

### Q: 支持哪些探针类型？

- kprobe / kretprobe
- tracepoint
- perf event
- uprobe（需自行扩展）
