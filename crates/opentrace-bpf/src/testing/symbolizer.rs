// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.

use std::borrow::Cow;
use std::collections::HashMap;

use crate::symbolizer::{ResolvedSymbol, SymbolizeInput, Symbolizer};

/// Mock Symbolizer 实现，用于测试
///
/// # 示例
///
/// ```rust
/// use opentrace_bpf::testing::MockSymbolizer;
/// use opentrace_bpf::symbolizers::{Symbolizer, SymbolizeInput, Source};
///
/// let symbolizer = MockSymbolizer::new()
///     .with_symbol(0x1000, "my_function", 0x1000);
///
/// let result = symbolizer.resolve(SymbolizeInput {
///     source: Source::Kernel,
///     addr: 0x1000,
/// });
/// assert_eq!(result.name.as_ref(), "my_function");
/// ```
pub struct MockSymbolizer {
    symbols: HashMap<u64, ResolvedSymbol<'static>>,
}

impl MockSymbolizer {
    /// 创建新的空 MockSymbolizer
    pub fn new() -> Self {
        Self {
            symbols: HashMap::new(),
        }
    }

    /// 添加符号映射
    pub fn with_symbol(mut self, addr: u64, name: &str, start_addr: u64) -> Self {
        self.symbols.insert(
            addr,
            ResolvedSymbol {
                name: Cow::Owned(name.to_string()),
                start_addr,
                offset: (addr - start_addr) as usize,
            },
        );
        self
    }

    /// 添加符号映射（带偏移量）
    pub fn with_symbol_and_offset(
        mut self,
        addr: u64,
        name: &str,
        start_addr: u64,
        offset: usize,
    ) -> Self {
        self.symbols.insert(
            addr,
            ResolvedSymbol {
                name: Cow::Owned(name.to_string()),
                start_addr,
                offset,
            },
        );
        self
    }
}

impl Default for MockSymbolizer {
    fn default() -> Self {
        Self::new()
    }
}

impl Symbolizer for MockSymbolizer {
    fn resolve(&self, input: SymbolizeInput) -> ResolvedSymbol<'_> {
        // 查找符号
        if let Some(symbol) = self.symbols.get(&input.addr) {
            return ResolvedSymbol {
                name: Cow::Owned(symbol.name.to_string()),
                start_addr: symbol.start_addr,
                offset: symbol.offset,
            };
        }

        // 默认返回 unknown
        ResolvedSymbol::unknown(input.addr, 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::symbolizer::Source;

    #[test]
    fn mock_symbolizer_returns_unknown_for_unmapped() {
        let symbolizer = MockSymbolizer::new();
        let result = symbolizer.resolve(SymbolizeInput {
            source: Source::Kernel,
            addr: 0x1000,
        });
        assert_eq!(result.name.as_ref(), "!0x1000");
    }

    #[test]
    fn mock_symbolizer_returns_mapped_symbol() {
        let symbolizer = MockSymbolizer::new().with_symbol(0x1000, "main", 0x1000);
        let result = symbolizer.resolve(SymbolizeInput {
            source: Source::Kernel,
            addr: 0x1000,
        });
        assert_eq!(result.name.as_ref(), "main");
        assert_eq!(result.start_addr, 0x1000);
        assert_eq!(result.offset, 0);
    }

    #[test]
    fn mock_symbolizer_calculates_offset() {
        let symbolizer = MockSymbolizer::new().with_symbol(0x1050, "func", 0x1000);
        let result = symbolizer.resolve(SymbolizeInput {
            source: Source::Kernel,
            addr: 0x1050,
        });
        assert_eq!(result.name.as_ref(), "func");
        assert_eq!(result.start_addr, 0x1000);
        assert_eq!(result.offset, 0x50);
    }

    #[test]
    fn mock_symbolizer_with_offset() {
        let symbolizer = MockSymbolizer::new().with_symbol_and_offset(0x1010, "func", 0x1000, 0x10);
        let result = symbolizer.resolve(SymbolizeInput {
            source: Source::Kernel,
            addr: 0x1010,
        });
        assert_eq!(result.name.as_ref(), "func");
        assert_eq!(result.start_addr, 0x1000);
        assert_eq!(result.offset, 0x10);
    }

    #[test]
    fn mock_symbolizer_different_sources_same_result() {
        let symbolizer = MockSymbolizer::new().with_symbol(0x1000, "func", 0x1000);

        // 不同 source 返回相同结果（因为没有 backend 区分）
        let r1 = symbolizer.resolve(SymbolizeInput {
            source: Source::Kernel,
            addr: 0x1000,
        });
        let r2 = symbolizer.resolve(SymbolizeInput {
            source: Source::CPid { pid: 1 },
            addr: 0x1000,
        });
        assert_eq!(r1.name, r2.name);
    }
}
