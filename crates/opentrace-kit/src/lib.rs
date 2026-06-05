// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.

mod server;

pub mod httpserver {
    pub use crate::server::Config as GenericServerConfig;
    pub use crate::server::Server as GenericHttpServer;
}
