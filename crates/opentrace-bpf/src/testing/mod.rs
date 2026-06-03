// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.
//! 测试工具模块
//!
//! 提供用于单元测试和集成测试的 mock 实现。
//!
//! 启用 `testing` feature 后可使用：
//! ```toml
//! [dev-dependencies]
//! opentrace-bpf = { path = "..", features = ["testing"] }
//! ```

mod collector;
mod exporter;
mod mock_profile;
mod mock_skbdrop;
mod mock_socket_tcp;
mod parser;
mod symbolizer;

pub use collector::MockCollector;
pub use exporter::MockExporter;
pub use mock_profile::{MockProfileCollector, make_profile_event, make_profile_event_both, make_profile_event_single};
pub use mock_skbdrop::{MockSkbdropCollector, make_skbdrop_event};
pub use mock_socket_tcp::{MockSocketTcpCollector, make_socket_tcp_event, make_socket_tcp_event_with_target};
pub use parser::{MockFrame, MockProtoParser};
pub use symbolizer::MockSymbolizer;
