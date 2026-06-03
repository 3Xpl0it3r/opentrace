use std::borrow::Cow;
// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.
use std::process::Command;

use super::ResolvedSymbol;

const SYM_DUMP_TOOLS: &str = "jallsyms";

// 因此Java的符号表是“流式数据”，不能跨时间复用，所以缓存下来没有意义，移除缓存todo
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

/// 需要跳过的非符号行前缀（jallsyms 输出里的状态/注释行）。
const SKIP_PREFIXES: &[&str] = &["#", "Attaching", "Starting", "Done"];

fn is_skippable_line(line: &str) -> bool {
    line.is_empty() || SKIP_PREFIXES.iter().any(|p| line.starts_with(p))
}

/// 解析单行符号：`<hex_addr> <hex_size> <name>`，格式不符或数字解析失败返回 None。
fn parse_symbol_line(line: &str) -> Option<(ResolvedSymbol<'static>, u64)> {
    let parts: Vec<&str> = line.splitn(3, ' ').collect();
    if parts.len() != 3 {
        return None;
    }
    let address = u64::from_str_radix(parts[0].trim(), 16).ok()?;
    let size = u64::from_str_radix(parts[1].trim(), 16).ok()?;
    let symbol = ResolvedSymbol {
        name: Cow::Owned(parts[2].trim().to_owned()),
        start_addr: address,
        offset: 0,
    };
    Some((symbol, size))
}

fn parse_symbol_output(input: &str) -> Vec<(ResolvedSymbol<'static>, u64)> {
    input
        .lines()
        .map(str::trim)
        .filter(|line| !is_skippable_line(line))
        .filter_map(parse_symbol_line)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::symbolizer::{Source, SymbolizeInput, Symbolizer};
    /* use rstest::rstest; */

    // ==================== symbol_contains 测试 ====================

    #[test]
    fn symbol_contains_at_exact_start_addr() {
        let symbol = ResolvedSymbol {
            name: Cow::Borrowed("func"),
            start_addr: 0x1000,
            offset: 0,
        };
        assert!(symbol_contains(&symbol, 0x100, 0x1000));
    }

    #[test]
    fn symbol_contains_within_range() {
        let symbol = ResolvedSymbol {
            name: Cow::Borrowed("func"),
            start_addr: 0x1000,
            offset: 0,
        };
        assert!(symbol_contains(&symbol, 0x100, 0x1050));
    }

    #[test]
    fn symbol_contains_at_end_boundary() {
        let symbol = ResolvedSymbol {
            name: Cow::Borrowed("func"),
            start_addr: 0x1000,
            offset: 0,
        };
        // addr == start_addr + size，应该是 false（不包含上界）
        assert!(!symbol_contains(&symbol, 0x100, 0x1100));
    }

    #[test]
    fn symbol_contains_before_start() {
        let symbol = ResolvedSymbol {
            name: Cow::Borrowed("func"),
            start_addr: 0x1000,
            offset: 0,
        };
        assert!(!symbol_contains(&symbol, 0x100, 0x0FFF));
    }

    #[test]
    fn symbol_contains_after_end() {
        let symbol = ResolvedSymbol {
            name: Cow::Borrowed("func"),
            start_addr: 0x1000,
            offset: 0,
        };
        assert!(!symbol_contains(&symbol, 0x100, 0x1200));
    }

    #[test]
    fn symbol_contains_zero_size() {
        let symbol = ResolvedSymbol {
            name: Cow::Borrowed("func"),
            start_addr: 0x1000,
            offset: 0,
        };
        // size == 0 时，只有 addr == start_addr 才返回 true
        assert!(symbol_contains(&symbol, 0, 0x1000));
        assert!(!symbol_contains(&symbol, 0, 0x1001));
    }

    // ==================== resolve_symbol 测试 ====================

    #[test]
    fn resolve_symbol_calculates_offset() {
        let symbol = ResolvedSymbol {
            name: Cow::Borrowed("func"),
            start_addr: 0x1000,
            offset: 0,
        };
        let resolved = resolve_symbol(&symbol, 0x1050);
        assert_eq!(resolved.name.as_ref(), "func");
        assert_eq!(resolved.start_addr, 0x1000);
        assert_eq!(resolved.offset, 0x50);
    }

    #[test]
    fn resolve_symbol_at_start_addr() {
        let symbol = ResolvedSymbol {
            name: Cow::Borrowed("func"),
            start_addr: 0x1000,
            offset: 0,
        };
        let resolved = resolve_symbol(&symbol, 0x1000);
        assert_eq!(resolved.offset, 0);
    }

    #[test]
    fn resolve_symbol_preserves_name() {
        let symbol = ResolvedSymbol {
            name: Cow::Owned("my_function".to_string()),
            start_addr: 0x1000,
            offset: 0,
        };
        let resolved = resolve_symbol(&symbol, 0x1010);
        assert_eq!(resolved.name.as_ref(), "my_function");
    }

    // ==================== is_skippable_line 测试 ====================

    #[test]
    fn is_skippable_line_empty() {
        assert!(is_skippable_line(""));
    }

    #[test]
    fn is_skippable_line_comment() {
        assert!(is_skippable_line("# this is a comment"));
        assert!(is_skippable_line("#"));
    }

    #[test]
    fn is_skippable_line_attaching() {
        assert!(is_skippable_line("Attaching to JVM"));
    }

    #[test]
    fn is_skippable_line_starting() {
        assert!(is_skippable_line("Starting dump"));
    }

    #[test]
    fn is_skippable_line_done() {
        assert!(is_skippable_line("Done"));
    }

    #[test]
    fn is_skippable_line_normal_symbol() {
        assert!(!is_skippable_line("1000 20 myFunc"));
    }

    #[test]
    fn is_skippable_line_partial_prefix_match() {
        // "Atta" 不是以 SKIP_PREFIXES 开头
        assert!(!is_skippable_line("Atta"));
    }

    // ==================== parse_symbol_line 测试 ====================

    #[test]
    fn parse_symbol_line_valid() {
        let result = parse_symbol_line("1a2b3c 100 myFunction").unwrap();
        assert_eq!(result.0.name.as_ref(), "myFunction");
        assert_eq!(result.0.start_addr, 0x1a2b3c);
        assert_eq!(result.1, 0x100);
    }

    #[test]
    fn parse_symbol_line_with_spaces_in_name() {
        let result = parse_symbol_line("1000 20 my function name").unwrap();
        assert_eq!(result.0.name.as_ref(), "my function name");
    }

    #[test]
    fn parse_symbol_line_uppercase_hex() {
        let result = parse_symbol_line("ABCDEF 10 test").unwrap();
        assert_eq!(result.0.start_addr, 0xabcdef);
    }

    #[test]
    fn parse_symbol_line_too_few_parts() {
        assert!(parse_symbol_line("1000 20").is_none());
    }

    #[test]
    fn parse_symbol_line_too_many_parts_only_two() {
        assert!(parse_symbol_line("1000").is_none());
    }

    #[test]
    fn parse_symbol_line_invalid_hex() {
        assert!(parse_symbol_line("not_hex 10 name").is_none());
    }

    #[test]
    fn parse_symbol_line_invalid_size() {
        assert!(parse_symbol_line("1000 not_size name").is_none());
    }

    #[test]
    fn parse_symbol_line_empty() {
        assert!(parse_symbol_line("").is_none());
    }

    // ==================== parse_symbol_output 批量测试 ====================

    #[test]
    fn parse_symbol_output_all_skippable_lines() {
        let output = "# comment\nAttaching\nStarting\nDone\n";
        assert!(parse_symbol_output(output).is_empty());
    }

    #[test]
    fn parse_symbol_output_mixed_content() {
        let output = "# header\n1000 10 func1\n2000 20 func2\nDone\n";
        let symbols = parse_symbol_output(output);
        assert_eq!(symbols.len(), 2);
        assert_eq!(symbols[0].0.name.as_ref(), "func1");
        assert_eq!(symbols[1].0.name.as_ref(), "func2");
    }

    #[test]
    fn parse_symbol_output_empty_input() {
        assert!(parse_symbol_output("").is_empty());
    }

    // ==================== 原有测试 ====================

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

    #[test]
    fn parses_symbol_output_while_skipping_status_lines() {
        let symbols = parse_symbol_output(
            "# comment\nAttaching to JVM\n1000 20 foo\nStarting dump\n2000 10 bar\nDone\n",
        );

        assert_eq!(symbols.len(), 2);
        assert_eq!(symbols[0].0.name.as_ref(), "foo");
        assert_eq!(symbols[0].1, 0x20);
        assert_eq!(symbols[1].0.start_addr, 0x2000);
    }

    #[test]
    fn rejects_invalid_symbol_lines() {
        assert!(parse_symbol_line("not enough").is_none());
        assert!(parse_symbol_line("zz 10 name").is_none());
        assert!(parse_symbol_line("10 zz name").is_none());
    }
}
