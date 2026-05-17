use std::borrow::Cow;
// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.
use std::process::Command;

use super::ResolvedSymbol;

const SYM_DUMP_TOOLS: &str = "jallsyms";

// todo  1. symbols 缓存下来 2. pid失效 3. symbols更新 等情况处理
pub struct JavaSymbolizer<'a> {
    /* tool: &'static str, */
    cache: Vec<(ResolvedSymbol<'a>, u64)>,
}

impl JavaSymbolizer<'_> {
    pub fn new(pid: u32) -> Self {
        println!("begin build java symbolizer");
        let result = Command::new(SYM_DUMP_TOOLS)
            .arg("-p")
            .arg(format!("{}", pid))
            .arg("--enable-native-library")
            .output()
            .expect("execute jallsyms failed");
        if !result.status.success() {
            eprintln!(
                "jallsyms failed: {}",
                String::from_utf8_lossy(&result.stderr)
            );
            return JavaSymbolizer {
                cache: Vec::default(),
            };
        }

        let stdout = String::from_utf8_lossy(&result.stdout);
        let mut symbols = parse_symbol_output(&stdout);
        symbols.sort_unstable_by_key(|(symbol, _)| symbol.start_addr);
        JavaSymbolizer { cache: symbols }
    }
}

impl<'a> super::Symbolizer for JavaSymbolizer<'a> {
    fn resolve(&self, input: super::SymbolizeInput) -> super::ResolvedSymbol<'a> {
        let index = match self
            .cache
            .binary_search_by_key(&input.addr, |(symbol, _)| symbol.start_addr)
        {
            Ok(index) => index,
            Err(0) => return super::ResolvedSymbol::unknown(input.addr, 0),
            Err(index) => index - 1,
        };

        match self.cache.get(index) {
            Some((symbol, size)) if symbol_contains(symbol, *size, input.addr) => {
                resolve_symbol(symbol, input.addr)
            }
            _ => super::ResolvedSymbol::unknown(input.addr, 0),
        }
    }

    fn update(&mut self, source: &super::Source) {
        if !self.cache.is_empty() {
            return;
        }
    }
}

fn symbol_contains(symbol: &ResolvedSymbol<'_>, size: u64, addr: u64) -> bool {
    if addr == symbol.start_addr {
        return true;
    }

    size > 0 && addr > symbol.start_addr && addr < symbol.start_addr.saturating_add(size)
}

fn resolve_symbol<'a>(symbol: &ResolvedSymbol<'a>, addr: u64) -> ResolvedSymbol<'a> {
    ResolvedSymbol {
        name: symbol.name.clone(),
        start_addr: symbol.start_addr,
        offset: addr.saturating_sub(symbol.start_addr) as usize,
    }
}

fn parse_symbol_output(input: &str) -> Vec<(ResolvedSymbol<'static>, u64)> {
    let mut symbols = Vec::new();

    for line in input.lines() {
        let line = line.trim();

        // 跳过空行、注释行、状态行
        if line.is_empty()
            || line.starts_with('#')
            || line.starts_with("Attaching")
            || line.starts_with("Starting")
            || line.starts_with("Done")
        {
            continue;
        }

        // 按空格拆成3部分：地址 大小 函数名
        let parts: Vec<&str> = line.splitn(3, ' ').collect();
        if parts.len() != 3 {
            continue;
        }

        // 解析地址（十六进制字符串 -> u64）
        let address = match u64::from_str_radix(parts[0].trim(), 16) {
            Ok(addr) => addr,
            Err(_) => continue,
        };

        // 解析符号大小（十六进制字符串 -> u64）
        let size = match u64::from_str_radix(parts[1].trim(), 16) {
            Ok(size) => size,
            Err(_) => continue,
        };

        let function_name = parts[2].trim();
        let symbol = ResolvedSymbol {
            name: Cow::Owned(function_name.to_owned()),
            start_addr: address,
            offset: 0,
        };

        symbols.push((symbol, size));
    }

    symbols
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::symbolizers::{Source, SymbolizeInput, Symbolizer};

    #[test]
    fn resolve_symbol_inside_function_body_uses_previous_entry() {
        let symbolizer = JavaSymbolizer {
            cache: vec![
                (
                    ResolvedSymbol {
                        start_addr: 0x1000,
                        name: Cow::Borrowed("foo"),
                        offset: 0,
                    },
                    0x20,
                ),
                (
                    ResolvedSymbol {
                        start_addr: 0x2000,
                        name: Cow::Borrowed("bar"),
                        offset: 0,
                    },
                    0x10,
                ),
            ],
        };

        let resolved = symbolizer.resolve(SymbolizeInput {
            source: Source::JavaPid { pid: 1 },
            addr: 0x1010,
        });

        assert_eq!(resolved.name.as_ref(), "foo");
        assert_eq!(resolved.start_addr, 0x1000);
        assert_eq!(resolved.offset, 0x10);
    }

    #[test]
    fn resolve_unknown_when_before_first_symbol() {
        let symbolizer = JavaSymbolizer {
            cache: vec![(
                ResolvedSymbol {
                    start_addr: 0x1000,
                    name: Cow::Borrowed("foo"),
                    offset: 0,
                },
                0x20,
            )],
        };

        let resolved = symbolizer.resolve(SymbolizeInput {
            source: Source::JavaPid { pid: 1 },
            addr: 0x0fff,
        });

        assert_eq!(resolved.start_addr, 0x0fff);
        assert_eq!(resolved.offset, 0);
    }
}
