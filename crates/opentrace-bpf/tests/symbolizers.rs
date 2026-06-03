// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.

//! 集成测试：symbolizers 模块
//!
//! 测试符号解析器

use opentrace_bpf::symbolizers::{ResolvedSymbol, Source, SymbolizerProvider};
use rstest::rstest;

// ==================== Source 测试 ====================

#[rstest]
#[case(Source::CPid { pid: 42 }, 42)]
#[case(Source::JavaPid { pid: 7 }, 7)]
#[case(Source::Kernel, 0)]
fn source_pid(#[case] source: Source, #[case] expected: u32) {
    assert_eq!(source.pid(), expected);
}

// ==================== ResolvedSymbol 测试 ====================

#[test]
fn resolved_symbol_unknown() {
    let symbol = ResolvedSymbol::unknown(0xabc, 3);
    assert_eq!(symbol.name.as_ref(), "!0xabc");
    assert_eq!(symbol.start_addr, 0xabc);
    assert_eq!(symbol.offset, 3);
}

#[test]
fn resolved_symbol_clone() {
    let symbol = ResolvedSymbol {
        name: std::borrow::Cow::Owned("test".to_string()),
        start_addr: 0x1000,
        offset: 10,
    };
    let cloned = symbol.clone();
    assert_eq!(cloned.name.as_ref(), "test");
    assert_eq!(cloned.start_addr, 0x1000);
    assert_eq!(cloned.offset, 10);
}

#[test]
fn resolved_symbol_equality() {
    let s1 = ResolvedSymbol {
        name: std::borrow::Cow::Borrowed("func"),
        start_addr: 0x1000,
        offset: 0,
    };
    let s2 = ResolvedSymbol {
        name: std::borrow::Cow::Borrowed("func"),
        start_addr: 0x1000,
        offset: 0,
    };
    assert_eq!(s1, s2);
}

// ==================== SymbolizerProvider 测试 ====================

#[test]
fn symbolizer_provider_default() {
    let provider = SymbolizerProvider::default();
    let _symbolizer = provider.get_symbolizer(&Source::Kernel);
}

#[test]
fn symbolizer_provider_returns_symbolizer_for_non_java() {
    let provider = SymbolizerProvider::default();

    // Kernel
    let _sym = provider.get_symbolizer(&Source::Kernel);

    // CPid
    let _sym = provider.get_symbolizer(&Source::CPid { pid: 1234 });
}

// ==================== MockSymbolizer 测试 ====================

// 这些测试在 testing feature 启用时运行
#[cfg(feature = "testing")]
mod mock_tests {
    use super::*;
    use opentrace_bpf::symbolizers::SymbolizeInput;
    use opentrace_bpf::symbolizers::Symbolizer;
    use opentrace_bpf::testing::MockSymbolizer;

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
    fn mock_symbolizer_different_sources_same_result() {
        let symbolizer = MockSymbolizer::new().with_symbol(0x1000, "func", 0x1000);

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
