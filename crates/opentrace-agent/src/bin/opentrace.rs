use opentrace_kit::httpserver::GenericHttpServer;

use opentrace_agent::agent::OpentraceAgent;
// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.

#[tokio::main]
async fn main() {
    println!("this is debug");
    match OpentraceAgent::new(GenericHttpServer::default()) {
        Ok(agent) => agent.run().await,
        Err(e) => {
            eprintln!("initial agent failed: {}", e);
            std::process::exit(-1)
        }
    }
}
