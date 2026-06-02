// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.
//
//

#[derive(Default)]
pub struct Config {
    pub bind_port: u32,
    pub bear_token: String,
    pub server_cert: String,
    pub server_cert_key: String,
    pub client_ca_certfile: String,
}

impl Config {
    pub(super) fn is_tls(&self) -> bool {
        !self.server_cert_key.is_empty() || !self.server_cert.is_empty()
    }
}

pub(super) struct SecurityConfig {
    pub(super) server_cert: String,
    pub(super) server_cert_key: String,
    pub(super) client_ca_certfile: String,
}

impl From<&Config> for SecurityConfig {
    fn from(value: &Config) -> Self {
        Self {
            server_cert: value.server_cert.clone(),
            server_cert_key: value.server_cert_key.clone(),
            client_ca_certfile: value.client_ca_certfile.clone(),
        }
    }
}
