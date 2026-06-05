// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.

mod net;
mod cpu;
mod macros;

use crate::{EbpfError, ProbeRegistry};

pub trait Collector: Send {
    fn poll(&mut self, internal: std::time::Duration) -> Result<(), EbpfError>;
    fn attach_probe(&mut self, probe_registry: &ProbeRegistry) -> Result<(), EbpfError>;
}

pub use cpu::{ProfileCollector, ProfileConfig, ProfileEvent, ProfileStackStorage};

// skbdrop
pub use net::{SkbdropCollector, SkbdropConfig, SkbdropEvent, SkbdropEventDefaultFormatter};

pub use net::{SocketTcpCollector, SocketTcpConfig, SocketTcpEvent, SocketTcpFormatter};
