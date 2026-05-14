// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.
//
use serde::Serialize;
use serde::ser::SerializeSeq;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ResolvedSymbol<'a> {
    // 函数名称
    pub name: &'a str,
    pub offset: u64,
}

pub trait SymbolResolver {
    fn resolve_addr(&self, addr: u64) -> Option<ResolvedSymbol<'_>>;
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Symbol {
    addr: u64,
    name: Box<str>,
}

impl Symbol {
    pub fn new(addr: u64, name: impl Into<Box<str>>) -> Self {
        Self {
            addr,
            name: name.into(),
        }
    }

    pub fn addr(&self) -> u64 {
        self.addr
    }

    pub fn name(&self) -> &str {
        self.name.as_ref()
    }
}

/// Address-sorted symbol table used for nearest-symbol lookup.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct SymbolTable {
    symbols: Vec<Symbol>,
}

impl SymbolTable {
    pub fn from_symbols(mut symbols: Vec<Symbol>) -> Self {
        symbols.sort_by_key(Symbol::addr);
        Self { symbols }
    }
}

impl SymbolResolver for SymbolTable {
    fn resolve_addr(&self, addr: u64) -> Option<ResolvedSymbol<'_>> {
        match self
            .symbols
            .binary_search_by_key(&addr, |symbol| symbol.addr)
        {
            Ok(idx) => Some(ResolvedSymbol {
                name: self.symbols[idx].name(),
                offset: 0,
            }),
            Err(idx) if idx > 0 => {
                let symbol = &self.symbols[idx - 1];
                Some(ResolvedSymbol {
                    name: symbol.name(),
                    offset: addr - symbol.addr(),
                })
            }
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StackFrame(pub u64);

impl StackFrame {
    pub fn addr(&self) -> u64 {
        self.0
    }

    pub fn resolve_with<'a, R>(&self, resolver: &'a R) -> Option<ResolvedSymbol<'a>>
    where
        R: SymbolResolver + ?Sized,
    {
        resolver.resolve_addr(self.0)
    }
}

pub struct Stack<'a, R: ?Sized> {
    frames: &'a [u64],
    resolver: &'a R,
}

impl<'a, R> Stack<'a, R>
where
    R: SymbolResolver + ?Sized,
{
    pub fn new(frames: &'a [u64], resolver: &'a R) -> Self {
        Self { frames, resolver }
    }
}

const UNKNOWN_FRAME: &str = "unknown";

struct SymbolizedFrame<'a> {
    name: &'a str,
    offset: u64,
}

impl<'a> From<ResolvedSymbol<'a>> for SymbolizedFrame<'a> {
    fn from(symbol: ResolvedSymbol<'a>) -> Self {
        Self {
            name: symbol.name,
            offset: symbol.offset,
        }
    }
}

impl Serialize for SymbolizedFrame<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.collect_str(&format_args!("{}({:#x})", self.name, self.offset))
    }
}

impl<R> Serialize for Stack<'_, R>
where
    R: SymbolResolver + ?Sized,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut seq = serializer.serialize_seq(Some(self.frames.len()))?;
        for addr in self.frames {
            let frame = StackFrame(*addr);
            if let Some(symbol) = frame.resolve_with(self.resolver) {
                seq.serialize_element(&SymbolizedFrame::from(symbol))?;
            } else {
                seq.serialize_element(&SymbolizedFrame {
                    name: UNKNOWN_FRAME,
                    offset: 0,
                })?;
            }
        }
        seq.end()
    }
}

