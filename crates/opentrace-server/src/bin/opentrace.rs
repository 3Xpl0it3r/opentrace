// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.

use clap::Parser;
use opentrace_server::OpenTraceServer;
use opentrace_server::config::Config;
use tracing_subscriber::EnvFilter;

/// OpenTrace Server - Management server with Web UI for MCP servers and Agents
#[derive(Parser)]
#[command(name = "opentrace-server", version, about)]
struct CliOptions {
    /// Server port
    #[arg(short, long)]
    port: Option<u16>,

    /// Database file path
    #[arg(short, long)]
    database: Option<String>,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("error")),
        )
        .init();

    let opts = CliOptions::parse();

    let mut config = Config::from_env();
    if let Some(database) = opts.database {
        config.database_path = database;
    }
    if let Some(port) = opts.port {
        config.port = port;
    }

    match OpenTraceServer::new(config) {
        Ok(server) => server.run().await,
        Err(e) => {
            eprintln!("initial server failed: {}", e);
            std::process::exit(1);
        }
    }
}
