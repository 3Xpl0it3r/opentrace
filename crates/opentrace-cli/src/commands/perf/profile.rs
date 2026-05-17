// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.
// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.

use std::collections::BTreeMap;
use std::fmt;
use std::time::Duration;

use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel};

use opentrace_bpf::collector::Collector;
use opentrace_bpf::collector::cpu::{ProfileCollector, ProfileEvent, ProfileSimpleExporter};
use opentrace_bpf::symbol::{Source, SymbolizeInput, Symbolizer, SymbolizerRegistry};
use opentrace_bpf::{Exporter, ProbeRegistry};

use crate::errors::CliError;
use crate::options::perf::ProfileOptions;

const KSTACK_FLAGS: u64 = 0xFFFFFFFF;

pub async fn run_as_profile(
    options: ProfileOptions,
    registry: &mut ProbeRegistry,
    object: &mut opentrace_bpf::CollectorObject,
) -> Result<(), CliError> {
    let mut symbolizer = SymbolizerRegistry::default();

    // 如果-1 或者或者不填写代表全采， 全采只dump kernel symbol(全进程dump 代价太大)
    // todo 后续离线可以支持, 或者有symbol server
    let source = match options.pid {
        Some(ref pid) if *pid > 0 => match options.language {
            Some(ref java) => Source::JavaPid { pid: *pid as u32 },
            None | Some(_) => Source::CPid { pid: *pid as u32 },
        },
        None | Some(_) => Source::Kernel,
    };
    symbolizer.update(&source);

    let (exporter, mut event_rx) = ProfileSimpleExporter::new();
    let mut collector = ProfileCollector::new(object, registry, options.into(), exporter)?;
    collector.attach_probe()?;

    let mut pre_handle_stack_storage = StacksStorage::default();
    let mut poll_interval = tokio::time::interval(Duration::from_millis(100));
    let sampling_timeout = tokio::time::sleep(Duration::from_secs(60));
    tokio::pin!(sampling_timeout);

    println!("开始采样.......");
    loop {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {
                break;
            }
            _ = &mut sampling_timeout => {
                break;
            }
            _ = poll_interval.tick() => {
                collector.poll(Duration::from_millis(0))?;
            }
            event = event_rx.recv() => {
                if let Some(event) = event {
                    let mut stack = event.0;
                    let mut kstack = event.1;
                    if kstack.len()!= 0 {
                        stack.push(KSTACK_FLAGS);
                        stack.append(&mut kstack);
                    }
                    pre_handle_stack_storage.insert(stack);
                }
            }
        }
    }
    println!("采样完成，开始处理......");
    let finnal_stack_storage = pre_handle_stack_storage.migrate_into_new_tree(source, &symbolizer);
    println!("{}", finnal_stack_storage);

    Ok(())
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
//otal samples: 426154
//L0_169192_(39%)           [u] _start
//  L1_169192_(100%)        [u] __libc_start_main
//    L2_169192_(100%)      [u] 0x7c6a0d42a1ca
//      L3_4917_(2%)        [u] _init
//

impl fmt::Display for StacksStorage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "total samples: {}", self.root.account)?;
        let header_width = self
            .root
            .children
            .values()
            .map(|child| child.max_header_width(self.root.account, 0))
            .max()
            .unwrap_or(0);

        for child in self.root.children.values() {
            child.fmt_with_parent(f, self.root.account, 0, header_width)?;
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
        header_width: usize,
    ) -> fmt::Result {
        let header = self.line_header(parent_total, depth);
        let stack_kind = if self.is_ustack { "[u]" } else { "[k]" };
        let stack_name = self
            .func_name
            .as_deref()
            .map_or_else(|| format!("-{}", self.stack_addr), ToOwned::to_owned);

        writeln!(f, "{header:<header_width$} {stack_kind} {stack_name}")?;

        for child in self.children.values() {
            child.fmt_with_parent(f, self.account, depth + 1, header_width)?;
        }

        Ok(())
    }

    fn max_header_width(&self, parent_total: u32, depth: usize) -> usize {
        self.children.values().fold(
            self.line_header(parent_total, depth).len(),
            |max_width, child| max_width.max(child.max_header_width(self.account, depth + 1)),
        )
    }

    fn line_header(&self, parent_total: u32, depth: usize) -> String {
        let percent = if parent_total == 0 {
            0
        } else {
            self.account * 100 / parent_total
        };

        format!(
            "{}L{}_{}_({}%)",
            "  ".repeat(depth),
            depth,
            self.account,
            percent
        )
    }
}

impl fmt::Display for Stacknode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.fmt_with_parent(f, self.account, 0, self.max_header_width(self.account, 0))
    }
}
