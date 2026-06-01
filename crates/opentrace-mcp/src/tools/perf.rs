// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.

use opentrace_bpf::exporter::SimpleUnboundChannelExporter;
use rmcp::model::{CallToolResult, Content};
use rmcp::{ErrorData, schemars};
use serde::Deserialize;
use tokio::sync::mpsc::UnboundedReceiver;
use tokio::time::Duration;

use opentrace_bpf::collector::Collector;
use opentrace_bpf::collector::cpu::{ProfileCollector, ProfileConfig, ProfileStackStorage};
use opentrace_bpf::symbol::{Source, SymbolizerProvider};

use crate::errors::MCPError;

const KSTACK_FLAGS: u64 = 0xFFFFFFFF;

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub(crate) struct PerfMcpToolParams {
    /// 针对指定pid采样, （不填写代表全采，0 代表只采集当前自己，>0
    /// 代表指定进程)值在（0-i32:MAX)之间
    #[schemars(
        description = "针对指定 pid 采样。不填写代表全采，0 代表只采集当前进程，>0 代表指定进程。值在 0 到 i32:MAX 之间。"
    )]
    #[serde(default)]
    pid: Option<i32>,

    /// 针对指定tid采样,必须提供pid (基于pid dumpsymbol)
    #[schemars(description = "针对指定 tid 采样。必须提供 pid（基于 pid dump symbol）。")]
    #[serde(default)]
    tid: Option<u32>,

    /// 绑定到指定 CPU 进行采样。设置为 -1 时在所有 CPU 上采样。
    #[schemars(description = "绑定到指定 CPU 进行采样。设置为 -1 时在所有 CPU 上采样。")]
    #[serde(default = "default_cpu")]
    cpu: i32,

    /// 符号解析扩展，支持针对指定类型的符号解析
    #[schemars(description = "符号解析扩展，支持针对指定类型的符号解析。")]
    #[serde(default)]
    language: Option<String>,

    #[schemars(
        description = "指定 eBPF perf 采样持续时间（秒）。超时后自动停止并返回已采集的栈样本结果。"
    )]
    #[serde(default = "default_timeout")]
    timeout: u32,
}

/// 默认 CPU 值 -1，表示所有 CPU
fn default_cpu() -> i32 {
    -1
}
fn default_timeout() -> u32 {
    60
}

impl PerfMcpToolParams {
    fn to_config(&self) -> ProfileConfig {
        let pid: i32 = self.pid.unwrap_or(-1);
        ProfileConfig {
            pid,
            tids: self.tid.map(|v| vec![v]),
            cpu: self.cpu,
            group_id: 0,
            custom_btf_path: None,
        }
    }

    fn timeout(&self) -> Duration {
        Duration::from_secs(self.timeout.max(1).into())
    }
}

pub(crate) async fn tool_handler(params: PerfMcpToolParams) -> Result<CallToolResult, ErrorData> {
    let mut object = opentrace_bpf::open_object_storage();
    let (exporter, event_rx) = SimpleUnboundChannelExporter::new();
    let mut collector = ProfileCollector::new(&mut object, params.to_config(), exporter)
        .map_err(MCPError::from)
        .map_err(ErrorData::from)?;
    collector
        .attach_probe()
        .map_err(MCPError::from)
        .map_err(ErrorData::from)?;

    // 开始采集数据
    let stack_storage = receive_profile_events(collector, event_rx, params.timeout())
        .await
        .map_err(ErrorData::from)?;
    // 符号解析
    let mut symbolizer_provider = SymbolizerProvider::default();
    let source = match (params.pid, params.language.as_deref()) {
        (Some(pid), Some("java")) => Source::JavaPid { pid: pid as u32 },
        (Some(pid), _) => Source::CPid { pid: pid as u32 },
        (None, _) => Source::Kernel,
    };
    symbolizer_provider.register(&source);
    let symbolizer = symbolizer_provider.get_symbolizer(&source);
    // 聚合
    let stack_storage = stack_storage.migrate_into_new_tree(source, symbolizer);

    Ok(CallToolResult::success(vec![Content::text(
        stack_storage.to_string(),
    )]))
}

async fn receive_profile_events(
    mut collector: impl Collector,
    mut rx: UnboundedReceiver<(Vec<u64>, Vec<u64>)>,
    timeout: Duration,
) -> Result<ProfileStackStorage, MCPError> {
    let mut stack_storage = ProfileStackStorage::default();
    let mut poll_interval = tokio::time::interval(Duration::from_millis(100));
    let sampling_timeout = tokio::time::sleep(timeout);
    tokio::pin!(sampling_timeout);

    loop {
        tokio::select! {
            _ = &mut sampling_timeout => {
                break;
            }
            _ = poll_interval.tick() => {
                collector.poll(Duration::from_millis(0))?;
            }
            event = rx.recv() => {
                let Some((mut stack, mut kstack)) = event else {
                    break;
                };

                if !kstack.is_empty() {
                    stack.push(KSTACK_FLAGS);
                    stack.append(&mut kstack);
                }
                stack_storage.insert(stack);
            }
        }
    }

    Ok(stack_storage)
}
