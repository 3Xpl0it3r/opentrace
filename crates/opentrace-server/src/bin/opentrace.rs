use opentrace_kit::httpserver::GenericHttpServer;
use opentrace_server::server::OpentraceServer;

// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.
//
#[tokio::main]
async fn main() {
    match OpentraceServer::new(GenericHttpServer::default()) {
        Ok(server) => server.run().await,
        Err(e) => {
            eprintln!("initial server failed: {}", e);
            std::process::exit(-1)
        }
    }
}
