// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.

mod agent_url;
pub mod config;
pub mod db;
pub mod errors;
pub mod handlers;
pub mod middleware;
pub mod models;
pub mod health;
pub mod server;
pub mod state;

pub use server::OpenTraceServer;
