// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.

mod net;
mod cpu;
mod macros;

use crate::EbpfError;

pub trait Collector {
    fn poll(&mut self, internal: std::time::Duration) -> Result<(), EbpfError>;
    fn attach_probe(&mut self) -> Result<(), EbpfError>;
}

pub use cpu::{ProfileCollector, ProfileConfig, ProfileEvent, ProfileStackEvent};

pub use net::{SkbdropCollector, SkbdropConfig, SkbdropEvent, SkbdropEventDefaultFormatter};

pub use net::{SocketDefaultFormatter, SocketTraceCollector, SocketTraceConfig, SocketTraceEvent};
