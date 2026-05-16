use super::types::{ResolvedSymbol, SymbolizeInput};

// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.
//
//
// SymbolRequest[#TODO] (shoule add some comments )


pub trait Symbolizer {
    fn resolve(&self, input: SymbolizeInput) -> ResolvedSymbol<'_>;
}
