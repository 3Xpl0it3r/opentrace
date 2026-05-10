// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.
use opentrace_bpf::EbpfError;
use rmcp::model::{ErrorCode, ErrorData};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum MCPError {
    #[error("IO Error: {0}")]
    IOErr(#[from] std::io::Error),

    #[error("Libbpf Error: {0}")]
    BpfErr(#[from] EbpfError),

    #[error("SDKMCP Error: {code:?}: {message}")]
    SDKMcpError {
        code: ErrorCode,
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error>>,
    },

    #[error("Other Error: {0}")]
    Other(String),
}

impl From<MCPError> for ErrorData {
    fn from(error: MCPError) -> Self {
        match error {
            MCPError::SDKMcpError {
                code,
                message,
                source: _,
            } => ErrorData {
                code,
                message: message.into(),
                data: None,
            },
            MCPError::BpfErr(e) => ErrorData {
                code: ErrorCode::INTERNAL_ERROR,
                message: e.to_string().into(),
                data: None,
            },
            MCPError::IOErr(e) => ErrorData {
                code: ErrorCode::INTERNAL_ERROR,
                message: e.to_string().into(),
                data: None,
            },
            MCPError::Other(e) => ErrorData {
                code: ErrorCode::INTERNAL_ERROR,
                message: e.into(),
                data: None,
            },
        }
    }
}
