// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.

use axum::Router;
use clap::Parser;

use opentrace_bpf::ProbeRegistry;
use opentrace_common::genericserver::{GenericHttpServer, GenericServerConfig};
use opentrace_mcp::{MCPError, McpServerOptions, OpentraceMcpServer};

#[tokio::main]
async fn main() -> Result<(), MCPError> {
    opentrace_bpf::env::setup_memlock_limit();

    let opts = McpServerOptions::parse();

    let mut server = GenericHttpServer::new(build_generic_server_config(opts));

    let probe_registry = ProbeRegistry::try_init()?;
    let mcp_service = OpentraceMcpServer::new_mcp_service(probe_registry);

    let mcp_router = Router::new().fallback_service(mcp_service);
    server
        .nest("/mcp", mcp_router)
        .map_err(|e| MCPError::Other(format!("{}", e)))?;

    if let Err(e) = server.run().await {
        eprintln!("{}", e);
        std::process::exit(-1);
    }

    Ok(())
}

fn build_generic_server_config(opts: McpServerOptions) -> GenericServerConfig {
    GenericServerConfig {
        bind_port: opts.port as u32,
        server_cert: opts.tls_cert.clone().unwrap_or_default(),
        server_cert_key: opts.tls_key.clone().unwrap_or_default(),
        client_ca_certfile: opts.client_ca.clone().unwrap_or_default(),
        bear_token: opts.bearer_token.unwrap_or_default(),
    }
}
