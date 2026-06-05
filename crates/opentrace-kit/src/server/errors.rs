// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.
use thiserror::Error as ThisError;
#[derive(Debug, ThisError)]
pub enum Error {
    #[error("IO Error: {0}")]
    Other(String),
}
