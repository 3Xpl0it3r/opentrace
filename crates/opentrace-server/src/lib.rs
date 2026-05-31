// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.

mod authentication;
pub mod config;
mod server;
mod errors;

pub use config::{AuthorizationConfig, Config as ServerConfig, SecurityConfig};
pub use server::Server as GenericServer;

pub use errors::ServerError;
