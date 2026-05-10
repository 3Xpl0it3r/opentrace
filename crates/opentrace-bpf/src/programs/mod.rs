// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.

pub(crate) mod net;
pub(crate) mod probe_registry;

use std::time::Duration;

use crate::EbpfError;

pub trait EbpfProgram {
    fn poll(&mut self, internal: Duration) -> Result<(), EbpfError>;
    fn attach_probe(&mut self) -> Result<(), EbpfError>;
}
