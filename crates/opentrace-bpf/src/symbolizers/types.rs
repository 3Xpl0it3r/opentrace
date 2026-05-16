use std::borrow::Cow;

// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.
#[derive(Eq, Hash, PartialEq)]
pub enum BackendKind {
    Blaze,
}

// 由于每次抓包Source 相对档次抓包就固定了(针对特定二进制程序)
#[derive(Clone)]
pub enum Source<'a> {
    Pid { pid: u32 },
    ELf { bin: Cow<'a, str> },
    Kernel,
}

impl Source<'_> {
    pub fn backend(&self) -> BackendKind {
        match self {
            Source::Pid { pid: _ } | Source::ELf { bin: _ } | Source::Kernel => BackendKind::Blaze,
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
            name: Cow::Owned(format!("0x{:x}", addr)),
            start_addr: addr,
            offset,
        }
    }
}
