// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.

use std::collections::HashMap;

use crate::symbolizers::balzesym::BalzeSymbolizer;

use super::symbolizer::Symbolizer;
use super::types::{BackendKind, Source, SymbolizeInput};

pub struct SymbolizerRegistry {
    registry: HashMap<BackendKind, Box<dyn Symbolizer>>,
}

impl Default for SymbolizerRegistry {
    fn default() -> Self {
        let mut registry: HashMap<BackendKind, Box<dyn Symbolizer>> = HashMap::new();
        registry.insert(BackendKind::Blaze, Box::new(BalzeSymbolizer::new()));

        Self { registry }
    }
}

// 如果找不到/或者没有注册symbolizer，那么则返回一个unknown ResolvedSymbol
impl Symbolizer for SymbolizerRegistry {
    fn resolve(&self, input: SymbolizeInput) -> super::ResolvedSymbol<'_> {
        let symbolizer = match input.source.backend() {
            BackendKind::Blaze => self.registry.get(&BackendKind::Blaze),
        };
        let symbolizer = symbolizer.unwrap();
        let sym_ed = symbolizer.resolve(input);
        sym_ed
    }
}
