# 示例：Exec Tracepoint

追踪 `sys_enter_execve` 系统调用。

## 文件结构

```
crates/opentrace-bpf/src/
├── bpf/exec.bpf.c                 # BPF 内核态程序
├── collectors/exec/
│   ├── mod.rs
│   ├── event.rs                   # Event 结构体
│   ├── collector.rs               # Collector 实现
│   └── formatter.rs               # Formatter 实现
└── lib.rs                         # 注册模块
```

---

## BPF 程序

```c
// crates/opentrace-bpf/src/bpf/exec.bpf.c

#include "vmlinux.h"
#include "libbpf/src/bpf_helpers.h"
#include "include/common.h"
#include "include/ebpf_map.h"

struct exec_event_t {
    u32 pid;
    u32 ppid;
    u64 timestamp;
    char comm[16];
    char filename[256];
};

BPF_PERCPU_ARRAY_DEF(event_heap, struct exec_event_t, 1);
BPF_PERF_EVENT_ARRAY_DEF(perf_events);

SEC("tracepoint/syscalls/sys_enter_execve")
int tp_sys_enter_execve(struct trace_event_raw_sys_enter *ctx) {
    u32 key = 0;
    struct exec_event_t *event = bpf_map_lookup_elem(&event_heap, &key);
    if (!event) return BPF_OK;

    __builtin_memset(event, 0, sizeof(*event));
    event->pid = bpf_get_current_pid_tgid() >> 32;
    event->timestamp = bpf_ktime_get_ns();
    bpf_get_current_comm(event->comm, sizeof(event->comm));

    bpf_perf_event_output(ctx, &perf_events, BPF_F_CURRENT_CPU, event, sizeof(*event));
    return BPF_OK;
}

char _license[] SEC("license") = "GPL";
```

---

## Rust 代码

### Event

```rust
// crates/opentrace-bpf/src/collectors/exec/event.rs

#[derive(Clone, Copy)]
#[repr(C)]
pub struct ExecEvent {
    pub pid: u32,
    pub ppid: u32,
    pub timestamp: u64,
    pub comm: [u8; 16],
    pub filename: [u8; 256],
}

impl ExecEvent {
    pub fn comm_str(&self) -> &str {
        std::str::from_utf8(&self.comm[..self.comm.iter().position(|&b| b == 0).unwrap_or(16)])
            .unwrap_or("?")
    }
}
```

### Collector

```rust
// crates/opentrace-bpf/src/collectors/exec/collector.rs

use crate::bpf::exec::{ExecSkel, ExecSkelBuilder};
use crate::collectors::macros::{define_collector, attach_tracepoint};
use crate::exporters::{Exporter, helper::load_and_dispatch};

define_collector!(ExecCollector, ExecSkel);

impl<'a> ExecCollector<'a> {
    pub fn new(
        object: &'a mut MaybeUninit<OpenObject>,
        registry: &'a ProbeRegistry,
        mut exporter: impl Exporter<ExecEvent> + 'a,
    ) -> Result<Self, EbpfError> {
        let skel = ExecSkelBuilder::default().open(object)?.load()?;
        let perf_buffer = PerfBufferBuilder::new(&skel.maps.perf_events)
            .sample_cb(move |_, data| load_and_dispatch::<ExecEvent, _>(data, &mut exporter))
            .build()?;
        Ok(Self { probe_registry: registry, skel, perf_buffer, _links: Vec::new() })
    }

    fn do_attach_probes(&mut self) -> Result<(), EbpfError> {
        attach_tracepoint!(self, "syscalls", tp_sys_enter_execve);
        Ok(())
    }
}
```

### Formatter

```rust
// crates/opentrace-bpf/src/collectors/exec/formatter.rs

pub struct ExecFormatter;

impl StreamFormatter<ExecEvent> for ExecFormatter {
    fn format<W: Write>(&self, w: &mut W, e: &ExecEvent) -> io::Result<()> {
        writeln!(w, "{:>6} {:<16} {}", e.pid, e.comm_str(), e.filename_str())
    }
}
```

### 注册

```rust
// crates/opentrace-bpf/src/collectors/exec/mod.rs
mod event;
mod collector;
mod formatter;

pub use event::ExecEvent;
pub use collector::ExecCollector;
pub use formatter::ExecFormatter;
```

```rust
// crates/opentrace-bpf/src/collectors/mod.rs 添加:
mod exec;
pub use exec::{ExecCollector, ExecEvent, ExecFormatter};
```

---

## CLI 使用

```rust
// crates/opentrace-cli/src/commands/exec.rs

pub fn run(registry: &mut ProbeRegistry, object: &mut CollectorObject) -> Result<()> {
    let exporter = DefaultStdoutExporter::new(ExecFormatter);
    let mut collector = ExecCollector::new(object, registry, exporter)?;

    println!("{:>6} {:<16} {}", "PID", "COMMAND", "FILE");
    collector.attach_probe()?;
    loop { collector.poll(Duration::from_millis(100))?; }
}
```

运行效果：

```bash
$ sudo opentrace exec
   PID COMMAND          FILE
  1234 bash             /bin/bash
  1235 ls               /bin/ls
```

---

## 关键点

| 要点 | 说明 |
|------|------|
| `#[repr(C)]` | Event 字段顺序必须与 BPF 一致 |
| `define_collector!` | 自动生成 Collector 结构体和 trait 实现 |
| `tp_` 前缀 | BPF 程序名必须以 `tp_` 开头 |
| `perf_events` | Map 名称固定 |
