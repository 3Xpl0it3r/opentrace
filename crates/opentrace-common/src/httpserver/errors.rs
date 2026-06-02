// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.
use thiserror::Error;
#[derive(Debug, Error)]
pub enum ServerError {
    #[error("IO Error: {0}")]
    Other(String),
}
