# 开发概览

本文档介绍 OpenTrace 的架构和开发流程。

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

## 核心组件

| 组件 | 作用 | 位置 |
|------|------|------|
| **Collector** | 用户态 eBPF 程序，负责挂载探针、采集内核数据 | `crates/opentrace-bpf/src/collectors/` |
| **Exporter** | 数据导出器，将采集的数据发送到目标（终端/ES/Kafka等） | `crates/opentrace-bpf/src/exporters/` |
| **Formatter** | 数据格式化器，将 Event 格式化为可读字符串 | `crates/opentrace-bpf/src/formatter.rs` |
| **Protocol** | 应用层协议解析器，将原始字节解析为结构化帧 | `crates/opentrace-bpf/src/protocols/` |

## 目录结构

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
│   │   ├── protocols/      # 协议解析器
│   │   ├── types/          # 公共类型
│   │   └── lib.rs
│   └── build.rs            # BPF 编译脚本
│
├── opentrace-cli/          # 命令行工具
│   └── src/
│       ├── commands/       # 命令实现
│       └── bin/
│
└── opentrace-server/       # 服务端
    └── src/
```

---

## 开发流程

### 创建新的 Collector

1. **定义 Event 结构体** - 从内核态传递的数据结构
2. **定义 Config 结构体** - 向 eBPF 程序传递的配置
3. **创建 Collector** - 使用 `define_collector!` 宏
4. **实现 Formatter** - 格式化 Event 数据
5. **注册模块** - 在 `lib.rs` 中导出
6. **CLI 集成** - 在 CLI 中使用

详细步骤请参考 [Exec Tracepoint 示例](/guide/examples/exec-tracepoint)。

### 扩展协议解析器

1. **实现 ParsedFrame** - 定义解析后的帧结构
2. **实现 ProtoParser** - 协议解析逻辑
3. **注册模块** - 在 `protocols/mod.rs` 中导出

详细步骤请参考 [协议扩展](/guide/development/protocol-extension)。

---

## 详细开发步骤

### 步骤 1: 定义 Event 结构体

Event 是从内核态传递到用户态的数据结构，需要 `#[repr(C)]` 确保内存布局一致。

```rust
// crates/opentrace-bpf/src/collectors/your_domain/your_event.rs

#[derive(Clone, Copy)]
#[repr(C)]
pub struct YourEvent {
    pub pid: u32,
    pub tid: u32,
    pub comm: [u8; 16],
    pub timestamp: u64,
    // ... 其他字段
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

### ProtoParser Trait

```rust
pub trait ProtoParser {
    type Output: ParsedFrame;
    fn parse(&self, data: &[u8], size: usize, verbose: bool) -> Option<Self::Output>;
    fn hash_id(&self, data: &[u8], size: usize) -> u32;
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

## 调试技巧

### 查看内核日志

```bash
sudo cat /sys/kernel/debug/tracing/trace_pipe
```

### 在 BPF 程序中添加调试输出

```c
bpf_printk("captured %d bytes from pid=%d", size, pid);
```

### 运行测试

```bash
cargo test -p opentrace-bpf
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
