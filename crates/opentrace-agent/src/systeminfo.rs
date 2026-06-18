// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.

use std::collections::HashMap;
use std::sync::OnceLock;

use axum::Json;
use axum::Router;
use axum::routing::get;
use serde::Serialize;

pub(crate) const ENDPOINT: &str = "/systeminfo";

const VERSION: &str = env!("CARGO_PKG_VERSION");
const BUILD_TIME: &str = "2026-06-20T00:00:00Z";
const OS: &str = std::env::consts::OS;
const ARCH: &str = std::env::consts::ARCH;

fn kernel_version() -> &'static str {
    static KERNEL: OnceLock<String> = OnceLock::new();
    KERNEL.get_or_init(|| {
        std::process::Command::new("uname")
            .arg("-r")
            .output()
            .ok()
            .and_then(|o| {
                if o.status.success() {
                    String::from_utf8(o.stdout)
                        .ok()
                        .map(|s| s.trim().to_string())
                } else {
                    None
                }
            })
            .unwrap_or_else(|| "unknown".to_string())
    })
}

fn get_collectors_info() -> HashMap<&'static str, &'static str> {
    let mut info = HashMap::new();
    info.insert("skbdrop", "通过 eBPF kprobe 跟踪内核 skb 丢包事件（kfree_skb_reason），返回丢包详情（IP 地址、端口、协议、进程信息、内核堆栈）。支持配置捕获包数量及超时时间。适用于网络丢包，网络不通等问题排查。");
    info.insert("perf", "通过 eBPF perf_event_open 采样 CPU 性能，捕获运行进程的堆栈跟踪（内核 + 用户空间），识别 CPU 热点和性能瓶颈。支持按进程 PID 过滤采样事件（设为 0 采样所有进程）、绑定到指定 CPU（设为 -1 在所有 CPU 上采样）。可指定采样持续时间（秒），超时后自动停止并返回已采集的栈样本结果。适用于 CPU 过高、性能瓶颈等问题排查。");
    info
}

#[derive(Serialize)]
struct SystemInfoResponse<'a> {
    version: &'static str,
    build_time: &'static str,
    os: &'static str,
    arch: &'static str,
    kernel: &'static str,
    collectors: HashMap<&'a str, &'a str>,
}

pub(crate) fn router() -> Router {
    Router::new().route("/", get(systeminfo_handler))
}

async fn systeminfo_handler() -> Json<SystemInfoResponse<'static>> {
    let collectors = get_collectors_info();
    Json(SystemInfoResponse {
        version: VERSION,
        build_time: BUILD_TIME,
        os: OS,
        arch: ARCH,
        kernel: kernel_version(),
        collectors,
    })
}
