// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.

mod skbdrop;
mod resource;
mod sink;

pub use resource::{ApiResource, ApiRouter};
pub use sink::{add_sink, list_sinks, remove_sink, update_sink};
pub use skbdrop::SkbdropResource;
