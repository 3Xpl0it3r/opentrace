// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.

use blazesym::symbolize::{self, Symbolizer};

use super::types::{ResolvedSymbol, Source, SymbolizeInput};

// BalzeSymbolizer[#TODO] (shoule add some comments )
pub struct BalzeSymbolizer {
    symbolizer: Symbolizer,
}

impl super::Symbolizer for BalzeSymbolizer {
    fn resolve(&self, input: SymbolizeInput) -> super::ResolvedSymbol<'_> {
        let SymbolizeInput { source, addr } = input;

        let source = match source {
            Source::CPid { pid } => {
                symbolize::source::Source::from(symbolize::source::Process::new(pid.into()))
            }
            Source::ELf { bin: _ } => todo!(),
            Source::Kernel => {
                symbolize::source::Source::Kernel(symbolize::source::Kernel::default())
            }
            Source::JavaPid { pid: _ } => todo!(),
        };
        let input = symbolize::Input::AbsAddr(addr);
        let sym_ed = self.symbolizer.symbolize_single(&source, input);

        match sym_ed {
            Ok(symbolize::Symbolized::Sym(sym)) => super::ResolvedSymbol {
                name: sym.name,
                start_addr: sym.addr,
                offset: sym.offset,
            },
            Err(_) | Ok(_) => ResolvedSymbol::unknown(addr, 0),
        }
    }
}

impl BalzeSymbolizer {
    pub fn new() -> Self {
        Self {
            symbolizer: Symbolizer::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::BalzeSymbolizer;

    #[test]
    fn constructs_symbolizer() {
        let _symbolizer = BalzeSymbolizer::new();
    }
}
