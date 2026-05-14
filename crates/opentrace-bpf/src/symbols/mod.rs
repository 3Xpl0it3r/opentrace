// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.

mod kernel;
mod resolver;

pub use resolver::*;

pub fn new_kernel_symbol() -> SymbolTable {
    kernel::load_symbols()
}
