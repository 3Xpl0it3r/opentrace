// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.
//
//

pub struct Config {
    pub bind_port: u32,
    pub security_config: SecurityConfig,
    pub authz: AuthorizationConfig,
}

#[derive(Default)]
pub struct SecurityConfig {
    pub server_cert: String,
    pub server_cert_key: String,
    pub client_ca_certfile: String,
}

#[derive(Default)]
pub struct AuthorizationConfig {
    pub bear_token: String,
}

impl SecurityConfig {
    pub(crate) fn is_tls(&self) -> bool {
        !self.server_cert_key.is_empty() && !self.server_cert.is_empty()
    }
}

// 默认配置, 当调用直接调用GenericServer::default的时候加载这个默认配置
impl Default for Config {
    fn default() -> Self {
        Self {
            bind_port: 80,
            security_config: SecurityConfig::default(),
            authz: AuthorizationConfig::default(),
        }
    }
}
