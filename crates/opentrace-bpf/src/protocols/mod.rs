use std::process::Output;

// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.
mod inet;
mod ether;
mod http;

pub trait ProtoParser {
    type Output<'a>;
    fn parse<'a>(&self, data: &'a [u8], size: usize) -> Self::Output<'a>;
}

pub mod ip_proto {
    pub use super::inet::*;
}

pub mod eth_proto {
    pub use super::ether::*;
}

pub mod app_protos {
    pub use super::http::{Frame as HttpFrame, Parser as HttpParser};
}
