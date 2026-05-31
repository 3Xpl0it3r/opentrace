// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.

use axum::Router;
use clap::Parser;

use opentrace_bpf::ProbeRegistry;
use opentrace_mcp::{MCPError, McpServerOptions, OpentraceMcpServer};
use opentrace_server::{AuthorizationConfig, GenericServer, SecurityConfig, ServerConfig};

#[tokio::main]
async fn main() -> Result<(), MCPError> {
    opentrace_bpf::env::setup_memlock_limit();

    let opts = McpServerOptions::parse();

    let mut server = GenericServer::new(build_generic_server_config(opts));

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

fn build_generic_server_config(opts: McpServerOptions) -> ServerConfig {
    ServerConfig {
        bind_port: opts.port as u32,
        security_config: SecurityConfig {
            server_cert: opts.tls_cert.clone().unwrap_or_default(),
            server_cert_key: opts.tls_key.clone().unwrap_or_default(),
            client_ca_certfile: opts.client_ca.clone().unwrap_or_default(),
        },
        authz: AuthorizationConfig {
            bear_token: opts.bearer_token.unwrap_or_default(),
        },
    }
}
