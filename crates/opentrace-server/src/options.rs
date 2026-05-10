// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.

use crate::config::GenericConfig;

#[derive(clap::Parser)]
pub struct ServerOptions {}

impl From<ServerOptions> for GenericConfig {
    fn from(_options: ServerOptions) -> Self {
        GenericConfig {
            bind_port: 9999,
            server_cert: "".into(),
            server_cert_key: "".into(),
            client_ca_certfile: "".into(),
            bearer_token: "".into(),
        }
    }
}
