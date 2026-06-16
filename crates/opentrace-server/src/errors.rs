// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.

use thiserror::Error;
pub enum ServerError {
    #[error("Other Error: {0}")]
    Other(String),
}
