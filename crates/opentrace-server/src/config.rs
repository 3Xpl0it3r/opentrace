// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.

#[derive(Default)]
pub struct GenericConfig {
    pub bind_port: u32,
    pub server_cert: String,
    pub server_cert_key: String,
    pub client_ca_certfile: String,
    pub bearer_token: String,
}
