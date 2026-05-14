// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.

use std::collections::BTreeMap;
use std::fmt::{self, Write};

// StackTree[#TODO] (shoule add some comments )
#[derive(Default)]
pub(super) struct StackTreeNode {
    stack_addr: u64,
    account: u32,
    stack_str: String,
    children: BTreeMap<u64, StackTreeNode>,
}

#[derive(Default)]
pub(super) struct StackTree {
    root: StackTreeNode,
}

impl StackTree {
    pub(super) fn insert(&mut self, stack: &[u64]) {
        self.root.insert(stack);
    }

    pub(super) fn print(&self) {
        print!("{}", self);
    }
}

impl fmt::Display for StackTree {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let total = self.root.children.values().map(|child| child.account).sum();
        self.root.fmt_with_total(f, total, 0)
    }
}

impl StackTreeNode {
    pub(super) fn insert(&mut self, stack: &[u64]) {
        let Some((addr, rest)) = stack.split_first() else {
            return;
        };

        let child = self.children.entry(*addr).or_insert_with(|| StackTreeNode {
            stack_addr: *addr,
            account: 0,
            stack_str: "".to_string(),
            ..Default::default()
        });
        child.account += 1;
        child.insert(rest);
    }

    fn fmt_with_total(&self, f: &mut fmt::Formatter<'_>, total: u32, depth: usize) -> fmt::Result {
        for child in self.children.values() {
            let percent = if total == 0 {
                0
            } else {
                child.account * 100 / total
            };
            let stack = if child.stack_str.is_empty() {
                format!("0x{:x}", child.stack_addr)
            } else {
                child.stack_str.clone()
            };

            f.write_str(&"  ".repeat(depth))?;
            writeln!(f, "{}({}%) {}", child.account, percent, stack)?;
            child.fmt_with_total(f, total, depth + 1)?;
        }

        Ok(())
    }
}
