// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.

use opentrace_kit::httpserver::GenericHttpServer;

use crate::errors::ServerError;

// Name[#TODO] (shoule add some comments )
pub struct OpentraceServer {
    server: GenericHttpServer,
}

impl OpentraceServer {
    pub fn new(server: GenericHttpServer) -> Result<Self, ServerError> {
        Ok(Self { server })
    }

    pub async fn run(self) {
        if let Err(err) = self.server.run().await {
            eprintln!("server exited: {err}");
        }
    }
}
