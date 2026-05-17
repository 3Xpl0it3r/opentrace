// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.

use std::collections::BTreeMap;
use std::fmt;

use rmcp::model::{CallToolResult, Content};
use rmcp::{ErrorData, schemars};
use serde::Deserialize;
use tokio::sync::mpsc::UnboundedReceiver;
use tokio::time::Duration;

use opentrace_bpf::ProbeRegistry;
use opentrace_bpf::collector::Collector;
use opentrace_bpf::collector::cpu::{ProfileCollector, ProfileConfig, ProfileSimpleExporter};
use opentrace_bpf::symbol::{Source, SymbolizeInput, Symbolizer, SymbolizerRegistry};

use crate::errors::MCPError;

const KSTACK_FLAGS: u64 = 0xFFFFFFFF;

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub(crate) struct PerfMcpToolParams {
    /// 按进程 PID 过滤采样事件。设置为 0 时采样所有进程的 CPU 活动。
    #[schemars(
        description = "Filter profiling samples by process PID. Set to 0 to sample all processes."
    )]
    #[serde(default)]
    pid: i32,

    /// 绑定到指定 CPU 进行采样。设置为 -1 时在所有 CPU 上采样。
    #[schemars(description = "Pin profiling to a specific CPU. Set to -1 to sample on all CPUs.")]
    #[serde(default = "default_cpu")]
    cpu: i32,

    /// 指定 eBPF perf 采样持续时间（秒）。超时后自动停止并返回已采集的栈样本结果。
    #[schemars(
        description = "Duration of eBPF perf sampling in seconds. Sampling stops automatically after timeout and returns collected stack samples."
    )]
    #[serde(default = "default_timeout")]
    timeout: u32,
}

/// 默认 CPU 值 -1，表示所有 CPU
fn default_cpu() -> i32 {
    -1
}
fn default_timeout() -> u32 {
    60
}

impl PerfMcpToolParams {
    fn to_config(&self) -> ProfileConfig {
        ProfileConfig {
            pid: self.pid,
            cpu: self.cpu,
            group_id: 0,
        }
    }

    fn timeout(&self) -> Duration {
        Duration::from_secs(self.timeout.max(1).into())
    }
}

pub(crate) async fn tool_handler(
    params: PerfMcpToolParams,
    probe_registry: &ProbeRegistry,
) -> Result<CallToolResult, ErrorData> {
    let mut object = opentrace_bpf::open_object_storage();
    let (exporter, event_rx) = ProfileSimpleExporter::new();
    let mut collector =
        ProfileCollector::new(&mut object, probe_registry, params.to_config(), exporter)
            .map_err(MCPError::from)
            .map_err(ErrorData::from)?;
    collector
        .attach_probe()
        .map_err(MCPError::from)
        .map_err(ErrorData::from)?;

    let stack_storage = receive_profile_events(collector, event_rx, params.timeout())
        .await
        .map_err(ErrorData::from)?;
    let stack_storage = stack_storage.migrate_into_new_tree(
        Source::CPid {
            pid: params.pid.max(0) as u32,
        },
        &SymbolizerRegistry::default(),
    );

    Ok(CallToolResult::success(vec![Content::text(
        stack_storage.to_string(),
    )]))
}

pub(crate) async fn receive_profile_events(
    mut collector: impl Collector,
    mut rx: UnboundedReceiver<(Vec<u64>, Vec<u64>)>,
    timeout: Duration,
) -> Result<StacksStorage, MCPError> {
    let mut stack_storage = StacksStorage::default();
    let mut poll_interval = tokio::time::interval(Duration::from_millis(100));
    let sampling_timeout = tokio::time::sleep(timeout);
    tokio::pin!(sampling_timeout);

    loop {
        tokio::select! {
            _ = &mut sampling_timeout => {
                break;
            }
            _ = poll_interval.tick() => {
                collector.poll(Duration::from_millis(0))?;
            }
            event = rx.recv() => {
                let Some((mut stack, mut kstack)) = event else {
                    break;
                };

                if !kstack.is_empty() {
                    stack.push(KSTACK_FLAGS);
                    stack.append(&mut kstack);
                }
                stack_storage.insert(stack);
            }
        }
    }

    Ok(stack_storage)
}

// StackBuffer[#TODO] (shoule add some comments )

// 存储采用B树，AI辅助写的, 模型gpt5.5
// 里面主要存储stack信息聚合统计， 打印到终端
#[derive(Default)]
struct StacksStorage {
    root: Stacknode,
}

impl StacksStorage {
    fn insert(&mut self, stacks: Vec<u64>) {
        if stacks.is_empty() {
            return;
        }

        self.root.account += 1;
        self.root.insert(&stacks, true);
    }

    fn migrate_into_new_tree(self, source: Source, resolver: &impl Symbolizer) -> Self {
        let mut storage = StacksStorage::default();
        storage.root.account = self.root.account;

        for node in self.root.children.into_values() {
            node.migrate_with_compact_into(&mut storage.root, source.clone(), resolver);
        }

        storage
    }
}

// 格式如下
//# [u] _start 169192(39%)
//## [u] __libc_start_main 169192(100%)
//### [u] 0x7c6a0d42a1ca 169192(100%)

impl fmt::Display for StacksStorage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for child in self.root.children.values() {
            child.fmt_with_parent(f, self.root.account, 0)?;
        }

        Ok(())
    }
}

impl IntoIterator for StacksStorage {
    type Item = Stacknode;
    type IntoIter = std::collections::btree_map::IntoValues<(bool, u64), Stacknode>;

    fn into_iter(self) -> Self::IntoIter {
        self.root.children.into_values()
    }
}

// Stacknode[#TODO] (shoule add some comments )
struct Stacknode {
    is_ustack: bool,
    stack_addr: u64,
    // 出现次数
    account: u32,
    // 函数名
    func_name: Option<String>,
    children: BTreeMap<(bool, u64), Stacknode>,
}

impl Default for Stacknode {
    fn default() -> Self {
        Self {
            stack_addr: 0,
            account: 0,
            is_ustack: false,
            func_name: None,
            children: BTreeMap::new(),
        }
    }
}

impl Stacknode {
    fn insert(&mut self, stacks: &[u64], is_ustack: bool) {
        let Some((stack_addr, stacks)) = stacks.split_first() else {
            return;
        };

        // KSTACK_FLAGS is only a user/kernel stack separator, not a stack frame.
        if *stack_addr == KSTACK_FLAGS {
            self.insert(stacks, false);
            return;
        }

        let child = self
            .children
            .entry((is_ustack, *stack_addr))
            .or_insert_with(|| Stacknode {
                is_ustack,
                stack_addr: *stack_addr,
                account: 0,
                func_name: None,
                children: BTreeMap::new(),
            });

        child.account += 1;
        child.insert(stacks, is_ustack);
    }

    fn migrate_with_compact_into(
        self,
        parent: &mut Stacknode,
        source: Source,
        symbolizer: &impl Symbolizer,
    ) {
        let symbol_source = if self.is_ustack {
            source.clone()
        } else {
            Source::Kernel
        };
        let sym_ed = symbolizer.resolve(SymbolizeInput {
            source: symbol_source,
            addr: self.stack_addr,
        });
        let stack_addr = sym_ed.start_addr;
        let func_name = sym_ed.name;

        if parent.stack_addr == stack_addr
            && parent.is_ustack == self.is_ustack
            && parent.stack_addr != 0
        {
            if parent.func_name.is_none() {
                parent.func_name = Some(func_name.into())
            }

            for node in self.children.into_values() {
                node.migrate_with_compact_into(parent, source.clone(), symbolizer);
            }

            return;
        }

        let child = parent
            .children
            .entry((self.is_ustack, stack_addr))
            .or_insert_with(|| Stacknode {
                stack_addr,
                is_ustack: self.is_ustack,
                account: 0,
                func_name: Some(func_name.into()),
                children: BTreeMap::new(),
            });

        child.account += self.account;

        for node in self.children.into_values() {
            node.migrate_with_compact_into(child, source.clone(), symbolizer);
        }
    }

    fn fmt_with_parent(
        &self,
        f: &mut fmt::Formatter<'_>,
        parent_total: u32,
        depth: usize,
    ) -> fmt::Result {
        let percent = if parent_total == 0 {
            0
        } else {
            self.account * 100 / parent_total
        };
        let stack_kind = if self.is_ustack { "[u]" } else { "[k]" };
        let stack_name = self
            .func_name
            .as_deref()
            .map_or_else(|| format!("{:#x}", self.stack_addr), ToOwned::to_owned);

        writeln!(
            f,
            "{} {stack_kind} {stack_name} {}({}%)",
            "#".repeat(depth + 1),
            self.account,
            percent
        )?;

        for child in self.children.values() {
            child.fmt_with_parent(f, self.account, depth + 1)?;
        }

        Ok(())
    }
}

impl fmt::Display for Stacknode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.fmt_with_parent(f, self.account, 0)
    }
}
