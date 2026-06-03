// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.

mod errors;
mod collector;
mod skeleton;
mod probe;
mod symbolizer;
mod formatter;
mod protocol;
mod exporter;

mod bpf;

pub mod env;
pub mod types;
pub mod utils;

#[cfg(feature = "testing")]
pub mod testing;

// 重新导出

pub mod exporters {
    pub use crate::exporter::Exporter;
    pub use crate::exporter::SimpleBoundChannelExpoter;
    pub use crate::exporter::SimpleUnboundChannelExporter;
    pub use crate::exporter::StreamWriterExpoter;
}

pub mod probes {
    pub use crate::probe::Registry as ProbeRegistry;
}

pub mod protocols {
    pub use crate::protocol::MessageType;
    pub use crate::protocol::ParsedFrame;
    pub use crate::protocol::ProtoParser;
    pub use crate::protocol::eth_proto;
    pub use crate::protocol::ip_proto;
    pub mod app_protos {
        pub use crate::protocol::app_protos::{HttpDirection, HttpFrame, HttpMethod, HttpParser};
    }
}

pub mod symbolizers {
    pub use crate::symbolizer::SymbolizerProvider;
    pub use crate::symbolizer::{ResolvedSymbol, Symbolizer};
    pub use crate::symbolizer::{Source, SymbolizeInput};
}

pub mod format {
    pub use crate::formatter::{StreamFormatter, StructeredFormatter};
}

pub mod collectors {
    pub use crate::collector::Collector;
    pub mod net {
        pub use crate::collector::{
            SkbdropCollector, SkbdropConfig, SkbdropEvent, SkbdropEventDefaultFormatter,
        };

        pub use crate::collector::{
            SocketTcpCollector, SocketTcpConfig, SocketTcpEvent, SocketTcpFormatter,
        };
    }

    pub mod cpu {
        pub use crate::collector::{
            ProfileCollector, ProfileConfig, ProfileEvent, ProfileStackStorage,
        };
    }
}

// 向后兼容
pub use errors::EbpfError;
pub use probes::ProbeRegistry;
pub use skeleton::{CollectorObject, open_object_storage};
