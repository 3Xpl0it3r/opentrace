// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.
mod inet;
mod ether;

pub mod ip_proto {
    pub use super::inet::*;
}

pub mod eth_proto {
    pub use super::ether::*;
}
