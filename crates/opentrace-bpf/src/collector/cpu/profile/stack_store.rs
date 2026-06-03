// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.
// StackBuffer

use std::collections::HashMap;
use std::fmt;

use crate::symbolizer::{Source, SymbolizeInput, Symbolizer};
const KSTACK_FLAGS: u64 = 0xFFFFFFFF;

// 存储采用B树，AI辅助写的, 模型gpt5.5
// 里面主要存储stack信息聚合统计， 打印到终端
#[derive(Default)]
pub struct StacksStorage {
    root: Stacknode,
}

impl StacksStorage {
    pub fn insert(&mut self, stacks: Vec<u64>) {
        if stacks.is_empty() {
            return;
        }

        self.root.account += 1;
        self.root.insert(&stacks, true);
    }

    pub fn merged(self, source: Source, resolver: &dyn Symbolizer) -> Self {
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
    type IntoIter = std::collections::hash_map::IntoValues<u64, Stacknode>;

    fn into_iter(self) -> Self::IntoIter {
        self.root.children.into_values()
    }
}

// Stacknode[#TODO] (shoule add some comments )
#[derive(Default)]
pub struct Stacknode {
    is_ustack: bool,
    stack_addr: u64,
    // 出现次数
    account: u32,
    // 函数名
    func_name: Option<String>,
    children: HashMap<u64, Stacknode>,
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
            .entry(*stack_addr)
            .or_insert_with(|| Stacknode {
                is_ustack,
                stack_addr: *stack_addr,
                account: 0,
                func_name: None,
                children: HashMap::default(),
            });

        child.account += 1;
        child.insert(stacks, is_ustack);
    }

    fn migrate_with_compact_into(
        self,
        parent: &mut Stacknode,
        source: Source,
        symbolizer: &dyn Symbolizer,
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
            .entry(stack_addr)
            .or_insert_with(|| Stacknode {
                is_ustack: self.is_ustack,
                stack_addr,
                account: 0,
                func_name: Some(func_name.into()),
                children: HashMap::new(),
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
        let percent = self
            .account
            .checked_mul(100)
            .and_then(|x| x.checked_div(parent_total))
            .unwrap_or(0);

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

#[cfg(test)]
mod tests {
    use std::borrow::Cow;

    use super::{KSTACK_FLAGS, StacksStorage};
    use crate::symbolizer::{ResolvedSymbol, Source, SymbolizeInput, Symbolizer};

    struct OffsetSymbolizer;

    impl Symbolizer for OffsetSymbolizer {
        fn resolve(&self, input: SymbolizeInput) -> ResolvedSymbol<'_> {
            let base = input.addr & !0xf;
            let prefix = match input.source {
                Source::Kernel => "k",
                _ => "u",
            };
            ResolvedSymbol {
                name: Cow::Owned(format!("{prefix}_{base:x}")),
                start_addr: base,
                offset: (input.addr - base) as usize,
            }
        }
    }

    #[test]
    fn insert_accumulates_total_samples() {
        let mut storage = StacksStorage::default();

        storage.insert(vec![0x11, 0x22]);
        storage.insert(vec![0x11, 0x33]);

        let output = storage.to_string();
        assert!(output.contains("total samples: 2"));
        assert!(output.contains("L0_2_(100%)"));
    }

    #[test]
    fn insert_switches_to_kernel_stack_after_separator() {
        let mut storage = StacksStorage::default();

        storage.insert(vec![0x10, KSTACK_FLAGS, 0x20]);

        let output = storage
            .merged(Source::CPid { pid: 1 }, &OffsetSymbolizer)
            .to_string();
        assert!(output.contains("[u] u_10"));
        assert!(output.contains("[k] k_20"));
    }

    #[test]
    fn merged_compacts_addresses_to_symbol_start() {
        let mut storage = StacksStorage::default();

        storage.insert(vec![0x11]);
        storage.insert(vec![0x12]);

        let output = storage
            .merged(Source::CPid { pid: 1 }, &OffsetSymbolizer)
            .to_string();
        assert!(output.contains("total samples: 2"));
        assert!(output.contains("L0_2_(100%)"));
        assert!(output.contains("[u] u_10"));
    }
}
