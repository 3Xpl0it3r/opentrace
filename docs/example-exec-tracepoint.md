# 完整示例：Exec Tracepoint 追踪

本文档演示如何从零开始创建一个基于 `sys_enter_execve` tracepoint 的 eBPF 追踪器，完整覆盖 BPF C 代码到 Rust 用户态代码。

## 目录结构

```
crates/opentrace-bpf/src/
├── bpf/
│   ├── exec.bpf.c                    # BPF 内核态程序
│   └── include/
│       └── exec_types.h              # 共享类型定义（可选）
│
├── collectors/
│   ├── mod.rs                        # 添加 exec 模块导出
│   └── exec/
│       ├── mod.rs                    # 模块入口
│       ├── event.rs                  # Event 结构体
│       ├── config.rs                 # Config 结构体
│       ├── collector.rs              # Collector 实现
│       └── formatter.rs              # Formatter 实现
│
└── lib.rs                            # 添加 pub mod 导出
```

---

## 第一步：BPF 内核态程序

### `crates/opentrace-bpf/src/bpf/exec.bpf.c`

```c
// Copyright 2026 opentrace Project Authors. Licensed under Apache-0.

#include "vmlinux.h"
#include "libbpf/src/bpf_helpers.h"
#include "libbpf/src/bpf_tracing.h"

#include "include/common.h"
#include "include/ebpf_map.h"

// 最大命令行参数长度
#define MAX_ARGS_LEN 256
#define MAX_ARG_COUNT 8

// ---------------------------------------------------------------------------
// 数据结构定义
// ---------------------------------------------------------------------------

// 用户态下发的过滤配置
struct config {
    u32 pid;           // 目标 PID，0 表示不过滤
    u32 uid;           // 目标 UID，0 表示不过滤
    u8 _pad[4];
};

// 上报到用户态的事件
struct exec_event_t {
    u32 pid;                           // 进程 PID
    u32 ppid;                          // 父进程 PID
    u32 uid;                           // 用户 UID
    u32 gid;                           // 用户 GID
    u64 timestamp;                     // 时间戳 (ns)
    char comm[TASK_COMM_LEN];          // 进程名
    char parent_comm[TASK_COMM_LEN];   // 父进程名
    char filename[256];                // 可执行文件路径
    char args[MAX_ARGS_LEN];           // 命令行参数（截断）
    u32 args_len;                      // 参数实际长度
};

// ---------------------------------------------------------------------------
// Maps
// ---------------------------------------------------------------------------

BPF_HASH_MAP_DEF(config_map, u8, struct config);
BPF_PERCPU_ARRAY_DEF(event_heap, struct exec_event_t, 1);
BPF_PERF_EVENT_ARRAY_DEF(perf_events);

// ---------------------------------------------------------------------------
// Helper: 读取文件名
// ---------------------------------------------------------------------------

static __always_inline void read_filename(struct filename *name,
                                          char *buf, size_t buf_len) {
    if (!name) {
        buf[0] = '\0';
        return;
    }
    // filename->name 是 const char *，指向实际路径
    const char *str = name->name;
    if (!str) {
        buf[0] = '\0';
        return;
    }
    bpf_probe_read_user_str(buf, buf_len, (void *)str);
}

// ---------------------------------------------------------------------------
// Helper: 读取 argv
// ---------------------------------------------------------------------------

static __always_inline u32 read_argv(char __user *const __user *argv,
                                     char *buf, size_t buf_len) {
    if (!argv || !buf)
        return 0;

    u32 offset = 0;
    u32 arg_idx = 0;

    #pragma unroll
    for (int i = 0; i < MAX_ARG_COUNT && offset < buf_len - 1; i++) {
        char *arg = NULL;
        long ret = bpf_probe_read_user(&arg, sizeof(arg), &argv[i]);
        if (ret < 0 || !arg)
            break;

        // 读取单个参数
        ssize_t len = bpf_probe_read_user_str(buf + offset,
                                               buf_len - offset,
                                               (void *)arg);
        if (len <= 0)
            break;

        // 替换 null terminator 为空格（除了最后一个参数）
        if (offset + len - 1 < buf_len) {
            buf[offset + len - 1] = ' ';
        }
        offset += len;
        arg_idx++;
    }

    // 确保字符串以 null 结尾
    if (offset > 0 && offset < buf_len) {
        buf[offset - 1] = '\0';
    }
    return offset;
}

// ---------------------------------------------------------------------------
// Tracepoint Handler
// ---------------------------------------------------------------------------

SEC("tracepoint/syscalls/sys_enter_execve")
int tp_sys_enter_execve(struct trace_event_raw_sys_enter *ctx) {
    // 读取配置
    u8 config_key = 0;
    struct config *cfg = bpf_map_lookup_elem(&config_map, &config_key);
    if (!cfg)
        return BPF_OK;

    // 获取当前进程信息
    u64 pid_tgid = bpf_get_current_pid_tgid();
    u32 pid = pid_tgid >> 32;
    u32 tid = (u32)pid_tgid;

    // 过滤：只关注主线程（pid == tid）
    if (pid != tid)
        return BPF_OK;

    // 过滤 PID
    if (cfg->pid != 0 && cfg->pid != pid)
        return BPF_OK;

    // 过滤 UID
    u64 uid_gid = bpf_get_current_uid_gid();
    u32 uid = (u32)uid_gid;
    if (cfg->uid != 0 && cfg->uid != uid)
        return BPF_OK;

    // 从 percpu array 获取 event buffer
    u32 event_key = 0;
    struct exec_event_t *event = bpf_map_lookup_elem(&event_heap, &event_key);
    if (!event)
        return BPF_OK;

    // 清零
    __builtin_memset(event, 0, sizeof(*event));

    // 填充基本信息
    event->pid = pid;
    event->uid = uid;
    event->gid = (u32)(uid_gid >> 32);
    event->timestamp = bpf_ktime_get_ns();

    // 获取当前进程名
    bpf_get_current_comm(event->comm, sizeof(event->comm));

    // 获取父进程信息
    struct task_struct *task = (struct task_struct *)bpf_get_current_task();
    struct task_struct *parent = NULL;
    bpf_probe_read_kernel(&parent, sizeof(parent), &task->real_parent);
    if (parent) {
        u32 ppid = 0;
        bpf_probe_read_kernel(&ppid, sizeof(ppid), &parent->tgid);
        event->ppid = ppid;
        bpf_probe_read_kernel_str(event->parent_comm, sizeof(event->parent_comm),
                                  &parent->comm);
    }

    // 读取 execve 的第一个参数：filename
    // ctx->args[0] 是 filename 指针
    struct filename *fname = NULL;
    bpf_probe_read_kernel(&fname, sizeof(fname), &ctx->args[0]);
    read_filename(fname, event->filename, sizeof(event->filename));

    // 读取 argv
    char __user *const __user *argv = NULL;
    bpf_probe_read_kernel(&argv, sizeof(argv), &ctx->args[1]);
    event->args_len = read_argv(argv, event->args, sizeof(event->args));

    // 上报事件
    bpf_perf_event_output(ctx, &perf_events, BPF_F_CURRENT_CPU,
                          event, sizeof(*event));

    return BPF_OK;
}

char _license[] SEC("license") = "GPL";
```

---

## 第二步：Rust 用户态代码

### 1. Event 结构体

#### `crates/opentrace-bpf/src/collectors/exec/event.rs`

```rust
// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.

use std::str;

/// 从内核态传递的 exec 事件
///
/// 字段顺序必须与 BPF 程序中的 `struct exec_event_t` 完全一致
#[derive(Clone, Copy)]
#[repr(C)]
pub struct ExecEvent {
    pub pid: u32,
    pub ppid: u32,
    pub uid: u32,
    pub gid: u32,
    pub timestamp: u64,
    pub comm: [u8; 16],         // TASK_COMM_LEN = 16
    pub parent_comm: [u8; 16],
    pub filename: [u8; 256],
    pub args: [u8; 256],        // MAX_ARGS_LEN
    pub args_len: u32,
}

impl ExecEvent {
    /// 获取进程名（去除 null 字节）
    pub fn comm_str(&self) -> &str {
        Self::trim_null(&self.comm)
            .and_then(|s| str::from_utf8(s).ok())
            .unwrap_or("<invalid>")
    }

    /// 获取父进程名
    pub fn parent_comm_str(&self) -> &str {
        Self::trim_null(&self.parent_comm)
            .and_then(|s| str::from_utf8(s).ok())
            .unwrap_or("<invalid>")
    }

    /// 获取可执行文件路径
    pub fn filename_str(&self) -> &str {
        Self::trim_null(&self.filename)
            .and_then(|s| str::from_utf8(s).ok())
            .unwrap_or("<invalid>")
    }

    /// 获取命令行参数
    pub fn args_str(&self) -> &str {
        let len = (self.args_len as usize).min(self.args.len());
        Self::trim_null(&self.args[..len])
            .and_then(|s| str::from_utf8(s).ok())
            .unwrap_or("<invalid>")
    }

    /// 去除尾部的 null 字节
    fn trim_null(data: &[u8]) -> Option<&[u8]> {
        let end = data.iter().position(|&b| b == 0).unwrap_or(data.len());
        if end == 0 {
            None
        } else {
            Some(&data[..end])
        }
    }
}
```

### 2. Config 结构体

#### `crates/opentrace-bpf/src/collectors/exec/config.rs`

```rust
// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.

use std::mem;

/// 用户态配置
#[derive(Debug, Clone)]
pub struct Config {
    /// 目标 PID，0 表示不过滤
    pub pid: u32,
    /// 目标 UID，0 表示不过滤
    pub uid: u32,
    /// 自定义 BTF 路径（用于不支持 BTF 的内核）
    pub custom_btf_path: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            pid: 0,
            uid: 0,
            custom_btf_path: None,
        }
    }
}

/// 传递给内核态的配置
///
/// 字段顺序必须与 BPF 程序中的 `struct config` 完全一致
#[repr(C)]
pub(crate) struct InnerConfig {
    pid: u32,
    uid: u32,
    _pad: [u8; 4],
}

impl InnerConfig {
    pub fn as_bytes(&self) -> &[u8] {
        let ptr = self as *const Self as *const u8;
        let len = mem::size_of_val(self);
        unsafe { std::slice::from_raw_parts(ptr, len) }
    }
}

impl From<Config> for InnerConfig {
    fn from(config: Config) -> Self {
        InnerConfig {
            pid: config.pid,
            uid: config.uid,
            _pad: [0; 4],
        }
    }
}
```

### 3. Collector 实现

#### `crates/opentrace-bpf/src/collectors/exec/collector.rs`

```rust
// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.

use std::mem::MaybeUninit;

use libbpf_rs::skel::{OpenSkel, SkelBuilder};
use libbpf_rs::{MapCore, MapFlags, OpenObject, PerfBufferBuilder};

use crate::bpf::exec::{ExecSkel, ExecSkelBuilder};
use crate::collectors::macros::{define_collector, attach_tracepoint};
use crate::exporters::{Exporter, helper::load_and_dispatch};
use crate::skeleton::with_custom_btf_open_opts;
use crate::{EbpfError, ProbeRegistry};

use super::config::{Config, InnerConfig};
use super::event::ExecEvent;

const CONFIG_KEY: u8 = 0;

// 使用宏定义 Collector 结构体并自动实现 Collector trait
//
// 宏展开后会生成：
// pub struct ExecCollector<'a> {
//     probe_registry: &'a ProbeRegistry,
//     skel: ExecSkel<'a>,
//     perf_buffer: PerfBuffer<'a>,
//     _links: Vec<Link>,
// }
//
// impl<'a> Collector for ExecCollector<'a> {
//     fn poll(...) { ... }
//     fn attach_probe(...) { self.do_attach_probes() }
// }
define_collector!(ExecCollector, ExecSkel);

impl<'a> ExecCollector<'a> {
    /// 创建新的 ExecCollector
    ///
    /// # 参数
    /// - object: BPF 对象存储（通常由 `open_object_storage()` 创建）
    /// - registry: 探针注册表，用于检测内核支持的探针
    /// - config: 配置参数（PID 过滤等）
    /// - exporter: 事件导出器（如 DefaultStdoutExporter）
    pub fn new(
        object: &'a mut MaybeUninit<OpenObject>,
        registry: &'a ProbeRegistry,
        config: Config,
        mut exporter: impl Exporter<ExecEvent> + 'a,
    ) -> Result<Self, EbpfError> {
        // 1. 加载 BPF skeleton
        let skel = match config.custom_btf_path {
            Some(ref path) => with_custom_btf_open_opts(path, |open_opts| {
                Ok(ExecSkelBuilder::default()
                    .open_opts(open_opts, object)?
                    .load()?)
            })?,
            None => ExecSkelBuilder::default().open(object)?.load()?,
        };

        // 2. 写入配置到 BPF map
        skel.maps
            .config_map
            .update(
                &CONFIG_KEY.to_ne_bytes(),
                Into::<InnerConfig>::into(config).as_bytes(),
                MapFlags::ANY,
            )
            .map_err(EbpfError::Libbpf)?;

        // 3. 创建 PerfBuffer，设置回调函数
        //
        // 回调函数签名固定为 `move |_cpu: i32, data: &[u8]|`
        // 使用 load_and_dispatch 将原始字节转换为 Event 并分发给 exporter
        let perf_buffer = PerfBufferBuilder::new(&skel.maps.perf_events)
            .sample_cb(move |_cpu: i32, data: &[u8]| {
                load_and_dispatch::<ExecEvent, _>(data, &mut exporter);
            })
            .build()?;

        Ok(Self {
            probe_registry: registry,
            skel,
            perf_buffer,
            _links: Vec::new(),
        })
    }

    /// 挂载 eBPF 探针
    ///
    /// 此方法由宏生成的 `attach_probe()` 调用
    fn do_attach_probes(&mut self) -> Result<(), EbpfError> {
        // 挂载 tracepoint: syscalls:sys_enter_execve
        //
        // 注意：宏要求 BPF 程序名必须以 `tp_` 开头
        // 宏会自动解析出 tracepoint 名称：`sys_enter_execve`
        attach_tracepoint!(self, "syscalls", tp_sys_enter_execve);

        Ok(())
    }
}
```

### 4. Formatter 实现

#### `crates/opentrace-bpf/src/collectors/exec/formatter.rs`

```rust
// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.

use std::io::{self, Write};

use crate::formatter::StreamFormatter;

use super::event::ExecEvent;

/// 默认格式化器（简洁模式，单行输出）
pub struct DefaultFormatter;

impl DefaultFormatter {
    pub fn new() -> Self {
        Self
    }
}

impl StreamFormatter<ExecEvent> for DefaultFormatter {
    fn format<W: Write>(&self, w: &mut W, event: &ExecEvent) -> io::Result<()> {
        writeln!(
            w,
            "{:>6} {:>6} {:<16} {} {}",
            event.pid,
            event.ppid,
            event.comm_str(),
            event.filename_str(),
            event.args_str()
        )
    }
}

/// 详细格式化器
pub struct VerboseFormatter;

impl VerboseFormatter {
    pub fn new() -> Self {
        Self
    }
}

impl StreamFormatter<ExecEvent> for VerboseFormatter {
    fn format<W: Write>(&self, w: &mut W, event: &ExecEvent) -> io::Result<()> {
        writeln!(w, "=== Exec Event ===")?;
        writeln!(w, "  PID:       {}", event.pid)?;
        writeln!(w, "  PPID:      {}", event.ppid)?;
        writeln!(w, "  UID:       {}", event.uid)?;
        writeln!(w, "  GID:       {}", event.gid)?;
        writeln!(w, "  Command:   {}", event.comm_str())?;
        writeln!(w, "  Parent:    {} ({})", event.parent_comm_str(), event.ppid)?;
        writeln!(w, "  File:      {}", event.filename_str())?;
        writeln!(w, "  Args:      {}", event.args_str())?;
        writeln!(w, "  Timestamp: {} ns", event.timestamp)?;
        writeln!(w, "==================")
    }
}

/// JSON 格式化器
pub struct JsonFormatter;

impl JsonFormatter {
    pub fn new() -> Self {
        Self
    }
}

impl StreamFormatter<ExecEvent> for JsonFormatter {
    fn format<W: Write>(&self, w: &mut W, event: &ExecEvent) -> io::Result<()> {
        write!(
            w,
            r#"{{"pid":{},"ppid":{},"uid":{},"gid":{},"comm":"{}","parent":"{}","file":"{}","args":"{}","ts":{}}}"#,
            event.pid,
            event.ppid,
            event.uid,
            event.gid,
            event.comm_str(),
            event.parent_comm_str(),
            event.filename_str(),
            event.args_str().replace('"', "\\\""),
            event.timestamp
        )
    }
}
```

### 5. 模块入口

#### `crates/opentrace-bpf/src/collectors/exec/mod.rs`

```rust
// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.

mod event;
mod config;
mod collector;
mod formatter;

pub use config::Config as ExecConfig;
pub use event::ExecEvent;
pub use collector::ExecCollector;
pub use formatter::{
    DefaultFormatter as ExecDefaultFormatter,
    VerboseFormatter as ExecVerboseFormatter,
    JsonFormatter as ExecJsonFormatter,
};
```

### 6. 注册模块

#### 修改 `crates/opentrace-bpf/src/collectors/mod.rs`

```rust
// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.

mod net;
mod cpu;
mod exec;    // <-- 新增
mod macros;

use crate::EbpfError;

pub trait Collector {
    fn poll(&mut self, internal: std::time::Duration) -> Result<(), EbpfError>;
    fn attach_probe(&mut self) -> Result<(), EbpfError>;
}

pub use cpu::{ProfileCollector, ProfileConfig, ProfileEvent, ProfileStackEvent};

pub use net::{SkbdropCollector, SkbdropConfig, SkbdropEvent, SkbdropEventDefaultFormatter};

pub use net::{SocketDefaultFormatter, SocketTraceCollector, SocketTraceConfig, SocketTraceEvent};

// 新增 exec 模块导出
pub use exec::{ExecCollector, ExecConfig, ExecEvent, ExecDefaultFormatter, ExecJsonFormatter};
```

#### 修改 `crates/opentrace-bpf/src/lib.rs`

```rust
// 在 pub mod collector 中添加 exec 模块

pub mod collector {
    pub use crate::collectors::Collector;
    
    pub mod net {
        pub use crate::collectors::{
            SkbdropCollector, SkbdropConfig, SkbdropEvent, SkbdropEventDefaultFormatter,
        };
        pub use crate::collectors::{
            SocketDefaultFormatter, SocketTraceCollector, SocketTraceConfig, SocketTraceEvent,
        };
    }

    pub mod cpu {
        pub use crate::collectors::{
            ProfileCollector, ProfileConfig, ProfileEvent, ProfileStackEvent,
        };
    }

    // 新增 exec 模块
    pub mod exec {
        pub use crate::collectors::{
            ExecCollector, ExecConfig, ExecEvent,
            ExecDefaultFormatter, ExecVerboseFormatter, ExecJsonFormatter,
        };
    }
}
```

---

## 第三步：CLI 使用

### 创建命令文件

#### `crates/opentrace-cli/src/commands/exec.rs`

```rust
// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.

use std::time::Duration;

use opentrace_bpf::ProbeRegistry;
use opentrace_bpf::collector::Collector;
use opentrace_bpf::collector::exec::{
    ExecCollector, ExecConfig, ExecDefaultFormatter, ExecVerboseFormatter, ExecJsonFormatter,
};
use opentrace_bpf::exporter::DefaultStdoutExporter;

use crate::errors::CliError;

/// exec 命令选项
pub struct ExecOptions {
    /// 目标 PID
    pub pid: u32,
    /// 目标 UID
    pub uid: u32,
    /// 详细输出
    pub verbose: bool,
    /// JSON 输出
    pub json: bool,
    /// 自定义 BTF 路径
    pub custom_btf_path: Option<String>,
}

impl Default for ExecOptions {
    fn default() -> Self {
        Self {
            pid: 0,
            uid: 0,
            verbose: false,
            json: false,
            custom_btf_path: None,
        }
    }
}

/// 运行 exec 追踪
pub fn run(
    registry: &mut ProbeRegistry,
    object: &mut opentrace_bpf::CollectorObject,
    options: ExecOptions,
) -> Result<(), CliError> {
    let config = ExecConfig {
        pid: options.pid,
        uid: options.uid,
        custom_btf_path: options.custom_btf_path,
    };

    // 根据输出格式选择 Formatter 并创建 Collector
    if options.json {
        // JSON 模式
        let formatter = ExecJsonFormatter::new();
        let exporter = DefaultStdoutExporter::new(formatter);
        let mut collector = ExecCollector::new(object, registry, config, exporter)?;
        
        collector.attach_probe()?;
        loop {
            let _ = collector.poll(Duration::from_millis(100));
        }
    } else if options.verbose {
        // 详细模式
        let formatter = ExecVerboseFormatter::new();
        let exporter = DefaultStdoutExporter::new(formatter);
        let mut collector = ExecCollector::new(object, registry, config, exporter)?;
        
        collector.attach_probe()?;
        loop {
            let _ = collector.poll(Duration::from_millis(100));
        }
    } else {
        // 简洁模式（默认）
        let formatter = ExecDefaultFormatter::new();
        let exporter = DefaultStdoutExporter::new(formatter);
        let mut collector = ExecCollector::new(object, registry, config, exporter)?;
        
        // 打印表头
        println!("{:>6} {:>6} {:<16} {} {}", "PID", "PPID", "COMMAND", "FILE", "ARGS");
        println!("{}", "-".repeat(80));
        
        collector.attach_probe()?;
        loop {
            let _ = collector.poll(Duration::from_millis(100));
        }
    }
}
```

---

## 运行效果

### 简洁模式（默认）

```bash
$ sudo opentrace exec
   PID   PPID COMMAND          FILE                ARGS
--------------------------------------------------------------------------------
  1234   1000 bash             /bin/bash           bash
  1235   1234 ls               /bin/ls             ls -la /tmp
  1236   1234 grep             /usr/bin/grep       grep foo bar.txt
```

### 详细模式

```bash
$ sudo opentrace exec --verbose
=== Exec Event ===
  PID:       1234
  PPID:      1000
  UID:       1000
  GID:       1000
  Command:   bash
  Parent:    bash (1000)
  File:      /bin/bash
  Args:      bash
  Timestamp: 123456789012345 ns
==================
=== Exec Event ===
  PID:       1235
  PPID:      1234
  UID:       1000
  GID:       1000
  Command:   ls
  Parent:    bash (1234)
  File:      /bin/ls
  Args:      ls -la /tmp
  Timestamp: 123456789123456 ns
==================
```

### JSON 模式

```bash
$ sudo opentrace exec --json
{"pid":1234,"ppid":1000,"uid":1000,"gid":1000,"comm":"bash","parent":"bash","file":"/bin/bash","args":"bash","ts":123456789012345}
{"pid":1235,"ppid":1234,"uid":1000,"gid":1000,"comm":"ls","parent":"bash","file":"/bin/ls","args":"ls -la /tmp","ts":123456789123456}
```

---

## 关键点总结

### 1. BPF 程序要点

| 要点 | 说明 |
|------|------|
| `SEC("tracepoint/syscalls/sys_enter_execve")` | 声明 tracepoint 挂载点 |
| `struct exec_event_t` | 与 Rust 的 `#[repr(C)]` 结构体字段顺序一致 |
| `BPF_PERF_EVENT_ARRAY_DEF(perf_events)` | 定义 perf event array，名字必须是 `perf_events` |
| `BPF_PERCPU_ARRAY_DEF(event_heap, ...)` | 使用 percpu array 避免栈大小限制 |
| `bpf_perf_event_output()` | 上报事件到用户态 |

### 2. Rust 代码要点

| 要点 | 说明 |
|------|------|
| `#[repr(C)]` | 确保内存布局与 BPF 一致 |
| `define_collector!(ExecCollector, ExecSkel)` | 自动生成结构体和 trait 实现 |
| `load_and_dispatch::<ExecEvent, _>()` | 固定的回调函数写法 |
| `attach_tracepoint!(self, "syscalls", tp_sys_enter_execve)` | BPF 程序名必须以 `tp_` 开头 |
| `impl StreamFormatter<ExecEvent>` | 实现格式化逻辑，Event 本身不实现 Debug/Display |
| `DefaultStdoutExporter::new(formatter)` | Exporter 持有 Formatter，负责输出 |

### 3. 命名约定

| 位置 | 命名规则 | 示例 |
|------|---------|------|
| BPF 程序名 | `tp_` + tracepoint 名 | `tp_sys_enter_execve` |
| Skeleton 模块 | BPF 文件名 | `exec.bpf.c` → `exec` |
| Skeleton 类型 | 首字母大写 + `Skel` | `ExecSkel`, `ExecSkelBuilder` |
| Map 名称 | 固定 | `config_map`, `perf_events`, `event_heap` |
