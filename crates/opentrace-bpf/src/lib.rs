// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.

mod bpf;
mod errors;
mod exporter;
mod collectors;
mod skeleton;
mod probes;
mod symbolizers;
mod formatter;
mod protocols;

pub mod env;
pub mod types;
pub mod utils;

// 重新导出
pub use errors::EbpfError;
pub use exporter::Exporter;
pub use probes::Registry as ProbeRegistry;
pub use skeleton::{CollectorObject, open_object_storage};

pub mod protocol {
    pub use crate::protocols::ProtoParser;
    pub use crate::protocols::eth_proto;
    pub use crate::protocols::ip_proto;
    pub mod appproto {
        pub use crate::protocols::app_protos::{HttpFrame, HttpParser};
    }
}

pub mod symbol {
    pub use crate::symbolizers::SymbolizerProvider;
    pub use crate::symbolizers::{ResolvedSymbol, Symbolizer};
    pub use crate::symbolizers::{Source, SymbolizeInput};
}

pub mod format {
    pub use crate::formatter::{StreamFormatter, StructeredFormatter};
}
//
pub mod collector {
    pub use crate::collectors::Collector;
    pub mod net {
        pub use crate::collectors::{
            SkbdropCollector, SkbdropConfig, SkbdropConsoleExpoter, SkbdropEvent,
            SkbdropEventDefaultFormatter,
        };

        pub use crate::collectors::{
            SocketDefaultExporter, SocketTraceCollector, SocketTraceConfig, SocketTraceEvent,
        };
    }

    pub mod cpu {
        pub use crate::collectors::{
            ProfileCollector, ProfileConfig, ProfileEvent, ProfileSimpleExporter,
        };
    }
}
