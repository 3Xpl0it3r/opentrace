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
        description = "通过 eBPF kprobe 跟踪内核 skb 丢包事件（kfree_skb_reason），返回丢包详情（IP 地址、端口、协议、进程信息、内核堆栈）。支持配置捕获包数量及超时时间。适用于网络丢包，网络不通等问题排查。"
    )]
    async fn skbdrop(
        &self,
        params: Parameters<tools::skbdrop::SkbdropMcpToolParams>,
    ) -> Result<CallToolResult, ErrorData> {
        tools::skbdrop::tool_handler(params.0, &self.probe_registry)
    }

    #[tool(
        description = "通过 eBPF perf_event_open 采样 CPU 性能，捕获运行进程的堆栈跟踪（内核 + 用户空间），识别 CPU 热点和性能瓶颈。支持按 PID 或 CPU 过滤。适用于 CPU 过高，性能瓶颈等问题排查。"
    )]
    async fn perf(
        &self,
        params: Parameters<tools::perf::PerfMcpToolParams>,
    ) -> Result<CallToolResult, ErrorData> {
        tools::perf::tool_handler(params.0, &self.probe_registry).await
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
