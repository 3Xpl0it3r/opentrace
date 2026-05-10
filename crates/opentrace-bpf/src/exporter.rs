// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.

use serde::Serialize;

/// Handles events emitted by an eBPF program.
pub trait Exporter<E: Send + Sized + Serialize + Clone> {
    fn handle(&mut self, event: E);

    fn load(&self, data: &[u8]) -> E {
        unsafe { std::ptr::read(data.as_ptr() as *const E) }
    }
}
