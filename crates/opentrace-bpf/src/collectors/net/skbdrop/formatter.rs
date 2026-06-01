use std::mem;

use crate::format::StreamFormatter;
use crate::symbol::{Source, SymbolizeInput, Symbolizer};
use crate::types::net::{AddrV4, AddrV6};

use super::Event;

// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.
pub struct DefaultFormatter<'a> {
    symbolizer: &'a dyn Symbolizer,
    source: Source<'a>,
}

impl<'a> DefaultFormatter<'a> {
    pub fn new(symbolizer: &'a dyn Symbolizer) -> Self {
        Self {
            symbolizer,
            source: Source::Kernel,
        }
    }
}

impl StreamFormatter<Event> for DefaultFormatter<'_> {
    fn format<W: std::io::Write>(&self, w: &mut W, event: &Event) -> Result<(), std::io::Error> {
        write_endpoints(w, event)?;
        writeln!(w, "reason: {}", event.drop_source_str())?;
        self.write_stack(w, event)?;
        writeln!(w, "{}", "---+---".repeat(10))
    }
}

/// 输出 "src:sport -> dst:dport" 一行，按 IP 版本走不同地址格式。
fn write_endpoints<W: std::io::Write>(w: &mut W, event: &Event) -> Result<(), std::io::Error> {
    let sport = u16::from_be(event.l4_info.sport);
    let dport = u16::from_be(event.l4_info.dport);
    match event.l3_info.ip_version {
        4 => writeln!(
            w,
            "{}:{} -> {}:{}",
            AddrV4::from(event.l3_info.saddr),
            sport,
            AddrV4::from(event.l3_info.daddr),
            dport,
        ),
        6 => writeln!(
            w,
            "{}:{} -> {}:{}",
            AddrV6::from(event.l3_info.saddr),
            sport,
            AddrV6::from(event.l3_info.daddr),
            dport,
        ),
        _ => writeln!(w, "0.0.0.0:{} -> 0.0.0.0:{}", sport, dport),
    }
}

impl DefaultFormatter<'_> {
    fn write_stack<W: std::io::Write>(
        &self,
        w: &mut W,
        event: &Event,
    ) -> Result<(), std::io::Error> {
        if event.stack_size <= 0 {
            return Ok(());
        }
        writeln!(w, "stack:")?;
        let stk_cnt = (event.stack_size as usize) / mem::size_of::<u64>();
        for addr in event.stack[..stk_cnt.min(event.stack.len())].iter() {
            let symb = self.symbolizer.resolve(SymbolizeInput {
                source: self.source.clone(),
                addr: *addr,
            });
            writeln!(w, "    {}", symb.name)?;
        }
        Ok(())
    }
}
