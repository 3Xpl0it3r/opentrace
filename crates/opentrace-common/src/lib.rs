// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.

mod httpserver;

pub mod genericserver {
    pub use crate::httpserver::Config as GenericServerConfig;
    pub use crate::httpserver::Server as GenericHttpServer;
}
