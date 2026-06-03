#![no_main]

use std::borrow::Cow;

use libfuzzer_sys::fuzz_target;
use opentrace_bpf::collector::cpu::ProfileStackStorage;
use opentrace_bpf::symbol::{ResolvedSymbol, Source, SymbolizeInput, Symbolizer};

/// 简单的 fuzzing 用 Symbolizer，不做真实符号解析。
struct FuzzSymbolizer;

impl Symbolizer for FuzzSymbolizer {
    fn resolve(&self, input: SymbolizeInput) -> ResolvedSymbol<'_> {
        // 用地址的低位字节区分用户/内核
        let prefix = match input.source {
            Source::Kernel => "k",
            _ => "u",
        };
        ResolvedSymbol {
            name: Cow::Owned(format!("{prefix}_{:x}", input.addr & 0xfff)),
            start_addr: input.addr & !0xf,
            offset: (input.addr & 0xf) as usize,
        }
    }
}

fuzz_target!(|data: &[u8]| {
    // 将输入分为若干段，每段 8 字节（一个 u64 栈地址）
    let chunks: Vec<u64> = data
        .chunks_exact(8)
        .map(|c| u64::from_ne_bytes([c[0], c[1], c[2], c[3], c[4], c[5], c[6], c[7]]))
        .collect();

    if chunks.is_empty() {
        return;
    }

    let mut storage = ProfileStackStorage::default();

    // 将地址按组插入（模拟多个栈采样）
    let mut pos = 0;
    while pos < chunks.len() {
        let group_size = 1 + (chunks[pos] as usize % 16); // 每组 1-16 个地址
        let end = (pos + group_size).min(chunks.len());
        let stack: Vec<u64> = chunks[pos..end].to_vec();
        storage.insert(stack);
        pos = end;
    }

    // Display 不能 panic
    let output = storage.to_string();
    assert!(!output.is_empty());

    // merged 不能 panic
    let storage2 = ProfileStackStorage::default();
    let merged = storage2.merged(Source::CPid { pid: 1 }, &FuzzSymbolizer);
    let _ = merged.to_string();
});
