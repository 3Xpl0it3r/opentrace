// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.

use clap::Parser;

use opentrace_bpf::ProbeRegistry;
use opentrace_mcp::OpentraceMcpServer;
use opentrace_server::{options::ServerOptions, server::GenericServer};

#[tokio::main]
async fn main() {
    opentrace_bpf::env::setup_memlock_limit();

    let opts = ServerOptions::parse();

    let probe_registry = ProbeRegistry::try_init().unwrap();

    let mcp = OpentraceMcpServer::new_mcp_service(probe_registry);

    let server = GenericServer::new(opts.into(), mcp);

    server.run().await;
}
