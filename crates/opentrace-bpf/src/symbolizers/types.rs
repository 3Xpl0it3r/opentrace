use std::borrow::Cow;

// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.
#[derive(Eq, Hash, PartialEq)]
pub enum BackendKind {
    Blaze,
    Java,
}

// 由于每次抓包Source 相对档次抓包就固定了(针对特定二进制程序)
#[derive(Clone)]
pub enum Source<'a> {
    CPid { pid: u32 },
    ELf { bin: Cow<'a, str> },
    JavaPid { pid: u32 },
    Kernel,
}

impl Source<'_> {
    pub fn backend(&self) -> BackendKind {
        match self {
            Source::CPid { pid: _ } | Source::ELf { bin: _ } | Source::Kernel => BackendKind::Blaze,
            Source::JavaPid { pid: _ } => BackendKind::Java,
        }
    }

    #[inline]
    pub fn pid(&self) -> u32 {
        match self {
            Source::CPid { pid } | Source::JavaPid { pid } => *pid,
            _ => 0,
        }
    }
}

// 解析输入参数
pub struct SymbolizeInput<'a> {
    pub source: Source<'a>,
    pub addr: u64,
}

// 解析后的symbol信息
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResolvedSymbol<'a> {
    // 函数名称
    pub name: Cow<'a, str>,
    // 函数起始地址i
    pub start_addr: u64,
    // 函数库名
    /* pub file_name: u64, */
    // 当前栈地址相对函数起始地址偏移量
    pub offset: usize,
}

impl ResolvedSymbol<'_> {
    pub fn unknown(addr: u64, offset: usize) -> Self {
        Self {
            name: Cow::Owned(format!("!0x{:x}", addr)),
            start_addr: addr,
            offset,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{BackendKind, ResolvedSymbol, Source};

    #[test]
    fn source_selects_expected_backend() {
        assert!(matches!(
            (Source::CPid { pid: 1 }).backend(),
            BackendKind::Blaze
        ));
        assert!(matches!(Source::Kernel.backend(), BackendKind::Blaze));
        assert!(matches!(
            (Source::JavaPid { pid: 1 }).backend(),
            BackendKind::Java
        ));
    }

    #[test]
    fn source_pid_returns_process_id_when_available() {
        assert_eq!((Source::CPid { pid: 42 }).pid(), 42);
        assert_eq!((Source::JavaPid { pid: 7 }).pid(), 7);
        assert_eq!(Source::Kernel.pid(), 0);
    }

    #[test]
    fn unknown_symbol_uses_hex_address_name() {
        let symbol = ResolvedSymbol::unknown(0xabc, 3);

        assert_eq!(symbol.name.as_ref(), "!0xabc");
        assert_eq!(symbol.start_addr, 0xabc);
        assert_eq!(symbol.offset, 3);
    }
}
