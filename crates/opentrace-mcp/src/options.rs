// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.

use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "opentrace")]
#[command(about = "OpenTrace MCP Server - eBPF-based network tracing and performance analysis")]
pub struct ServerOptions {
    /// Server bind port
    #[arg(short, long, default_value = "8080")]
    pub port: u16,

    /// Bearer token for authentication (optional)
    #[arg(long)]
    pub bearer_token: Option<String>,

    /// TLS server certificate file path (optional, enables HTTPS)
    #[arg(long)]
    pub tls_cert: Option<String>,

    /// TLS server private key file path (optional, required if tls_cert is set)
    #[arg(long)]
    pub tls_key: Option<String>,

    /// Client CA certificate file path (optional, enables mTLS)
    #[arg(long)]
    pub client_ca: Option<String>,
}
