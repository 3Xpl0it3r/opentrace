// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.

use rmcp::ErrorData;
use rmcp::{
    ServerHandler,
    handler::server::{tool::ToolRouter, wrapper::Parameters},
    model::{CallToolResult, Implementation, ProtocolVersion, ServerCapabilities, ServerInfo},
    tool, tool_handler, tool_router,
    transport::{
        StreamableHttpService, streamable_http_server::session::local::LocalSessionManager,
    },
};

use opentrace_bpf::ProbeRegistry;

use crate::tools;

#[derive(Clone)]
pub struct OpentraceMcpServer {
    tool_router: ToolRouter<Self>,
    probe_registry: ProbeRegistry,
}

#[tool_router]
impl OpentraceMcpServer {
    pub fn new_mcp_service(probe_registry: ProbeRegistry) -> StreamableHttpService<Self> {
        let mcpsvr = Self {
            tool_router: Self::tool_router(),
            probe_registry,
        };
        StreamableHttpService::new(
            move || Ok(mcpsvr.clone()),
            LocalSessionManager::default().into(),
            Default::default(),
        )
    }

    #[tool(
        description = "Trace kernel skb drop events via eBPF kprobe on kfree_skb_reason, returning dropped packet details (IP addresses, ports, protocol, process info, kernel stack). Configurable packet count and timeout."
    )]
    async fn skbdrop(
        &self,
        params: Parameters<tools::skbdrop::SkbdropMcpToolParams>,
    ) -> Result<CallToolResult, ErrorData> {
        tools::skbdrop::tool_handler(params.0, &self.probe_registry)
    }
}

#[tool_handler]
impl ServerHandler for OpentraceMcpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::LATEST,
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            server_info: Implementation::from_build_env(),
            instructions: Some("Trace and inspect Linux networking events via eBPF.".to_string()),
        }
    }
}
