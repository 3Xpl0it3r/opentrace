// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.

mod bpf;
mod errors;
mod exporter;
mod collectors;
mod skeleton;
mod probes;
mod symbols;
mod formatter;

pub mod env;
pub mod protocols;
pub mod types;
pub mod utils;

// 重新导出
pub use errors::EbpfError;
pub use exporter::Exporter;
pub use probes::Registry as ProbeRegistry;
pub use skeleton::{CollectorObject, open_object_storage};

pub mod symbol {
    pub use crate::symbols::new_kernel_symbol;
    pub use crate::symbols::{ResolvedSymbol, StackFrame, Symbol, SymbolResolver, SymbolTable};
}

pub mod format {
    pub use crate::formatter::{Formatter, JsonFormatter};
}
//
pub mod collector {
    pub use crate::collectors::Collector;
    pub mod net {
        pub use crate::collectors::{
            SkbdropCollector, SkbdropConfig, SkbdropConsoleExpoter, SkbdropEvent,
            SkbdropEventDefaultFormatter,
        };
    }
    pub mod cpu {
        pub use crate::collectors::{
            ProfileCollector, ProfileConfig, ProfileConsoleExporter, ProfileEvent,ProfileFoldedFormatter,
        };
    }
}
