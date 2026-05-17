// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.

use std::borrow::Cow;
use std::collections::HashMap;

use crate::symbolizers::balzesym::BalzeSymbolizer;

use super::java::JavaSymbolizer;
use super::symbolizer::Symbolizer;
use super::types::{BackendKind, Source, SymbolizeInput};

pub struct SymbolizerProvider<'a> {
    balze: BalzeSymbolizer,
    java: Option<JavaSymbolizer<'a>>,
}

impl Default for SymbolizerProvider<'_> {
    fn default() -> Self {
        Self {
            balze: BalzeSymbolizer::new(),
            java: None,
        }
    }
}

impl SymbolizerProvider<'_> {
    pub fn register(&mut self, source: &Source) {
        match source {
            Source::JavaPid { pid } => {
                self.java = Some(JavaSymbolizer::new(*pid));
            }
            _ => {}
        }
        return;
    }

    pub fn get_symbolizer(&self, source: &Source) -> &dyn Symbolizer {
        match source {
            Source::JavaPid { pid: _ } if let Some(ref symbolizer) = self.java => symbolizer,
            _ => &self.balze,
        }
    }
}

/* impl<'a> Symbolizer for SymbolizerProvider<'a> {
    fn resolve(&self, input: SymbolizeInput) -> super::ResolvedSymbol<'a> {
        let symbolizer = match input.source.backend() {
            BackendKind::Blaze => self.providers.get(&BackendKind::Blaze),
            BackendKind::Java => self.providers.get(&BackendKind::Java),
        };
        let symbolizer = symbolizer.unwrap();
        let sym_ed = symbolizer.resolve(input);
        sym_ed
        todo!()
    }
} */
