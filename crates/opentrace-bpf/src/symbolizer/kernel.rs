// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.
use std::fs::File;
use std::io::{BufRead, BufReader};

use super::{Symbol, SymbolTable};

const KALLSYMS_PATH: &str = "/proc/kallsyms";

pub fn load_symbols() -> SymbolTable {
    load_symbols_from(KALLSYMS_PATH)
}

fn load_symbols_from(path: &str) -> SymbolTable {
    let file = match File::open(path) {
        Ok(file) => file,
        Err(_) => return SymbolTable::default(),
    };
    let reader = BufReader::new(file);

    let mut symbols = Vec::new();

    for line in reader.lines() {
        let Ok(line) = line else {
            continue;
        };
        if let Some(symbol) = parse_kallsyms_line(&line) {
            symbols.push(symbol);
        }
    }

    SymbolTable::from_symbols(symbols)
}

fn parse_kallsyms_line(line: &str) -> Option<Symbol> {
    let mut parts = line.split_whitespace();
    let addr = u64::from_str_radix(parts.next()?, 16).ok()?;
    if addr == 0 {
        return None;
    }

    let symbol_type = parts.next()?.as_bytes().first()?.to_ascii_lowercase();
    if !matches!(symbol_type, b't' | b'w') {
        return None;
    }

    let name = parts.next()?;

    Some(Symbol::new(addr, name.to_owned()))
}
