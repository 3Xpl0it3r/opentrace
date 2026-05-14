// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.

use super::ProfileEvent;
use crate::format::Formatter;
use crate::symbol::{StackFrame, SymbolResolver};

// 以 "fname1:fname2:fname3...." 输出出去, 这种格式可以用来生成火焰图
pub struct FoldedFormatter;

impl Formatter<ProfileEvent> for FoldedFormatter {
    fn format<W: std::io::Write, R: crate::symbol::SymbolResolver>(
        &self,
        w: &mut W,
        event: &ProfileEvent,
        resolver: &R,
    ) -> std::io::Result<()> {
        write_folded_stack(w, &event.kstack, event.kstack_sz, resolver)?;
        write_folded_stack(w, &event.ustack, event.ustack_sz, resolver)
    }
}

fn write_folded_stack<W, R>(
    w: &mut W,
    stack: &[u64],
    stack_size: i64,
    resolver: &R,
) -> std::io::Result<()>
where
    W: std::io::Write,
    R: SymbolResolver,
{
    let count = if stack_size > 0 {
        ((stack_size as usize) / std::mem::size_of::<u64>()).min(stack.len())
    } else {
        0
    };

    let mut first = true;
    for addr in stack[..count].iter() {
        if !first {
            write!(w, ":")?;
        }

        let frame = StackFrame(*addr);
        if let Some(symbol) = frame.resolve_with(resolver) {
            write!(w, "{}", symbol.name)?;
        } else {
            write!(w, "{addr:#x}")?;
        }

        first = false;
    }

    if !first {
        writeln!(w, " 1")?;
    }

    Ok(())
}
