// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.

use crate::symbolizer::balzesym::BalzeSymbolizer;

use super::Symbolizer;
use super::java::JavaSymbolizer;
use super::types::Source;

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
        // clippy 规则，先single，后面有其他再修正
        if let Source::JavaPid { pid } = source {
            self.java = Some(JavaSymbolizer::new(*pid));
        }
    }

    pub fn get_symbolizer(&self, source: &Source) -> &dyn Symbolizer {
        match source {
            Source::JavaPid { pid: _ } if let Some(ref symbolizer) = self.java => symbolizer,
            _ => &self.balze,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::SymbolizerProvider;
    use crate::symbolizer::Source;

    #[test]
    fn default_provider_returns_symbolizer_for_non_java_sources() {
        let provider = SymbolizerProvider::default();

        let _symbolizer = provider.get_symbolizer(&Source::Kernel);
    }
}
