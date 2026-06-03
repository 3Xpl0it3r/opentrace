// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.

mod balzesym;
mod provider;
mod types;
mod java;

pub use provider::SymbolizerProvider;
pub use types::{ResolvedSymbol, Source, SymbolizeInput};

pub trait Symbolizer {
    fn resolve(&self, input: SymbolizeInput) -> ResolvedSymbol<'_>;
}
