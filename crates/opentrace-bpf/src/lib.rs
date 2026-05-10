// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.

mod bpf;
mod errors;
mod exporter;
mod programs;
pub mod consts;
pub mod utils;

pub use errors::EbpfError;
pub use exporter::Exporter;
pub use programs::{EbpfProgram, probe_registry::ProbeRegistry};

pub mod skel {
    pub use crate::bpf::skbdrop::SkbdropSkelBuilder;
    // 重新导出OpenSkel和SkelBuilder，其他的库就无需在重新导入libbpf_rs库了
    pub use libbpf_rs::skel::{OpenSkel, SkelBuilder};
}

// 重新导出
pub mod prog {
    pub mod net {
        pub use crate::programs::net::skbdrop::{
            Config as SkbdropConfig, DefaultExporter as SkbdropDefaultExporter,
            Event as SkbdropEvent, Program as SkbdropProgram,
        };
    }
}
