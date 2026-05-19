// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.

use clap::{Args, Parser, Subcommand};

use crate::config::Config;

#[derive(Parser)]
pub struct ServerOptions {}

#[derive(Parser)]
pub struct CliOptions {
    /// 自定义btf 路径(如果内核不支持btf, 可以通过custom-btf-path 来指定btf来加载)
    #[arg(long = "custom-btf-path", global = true)]
    pub custom_btf_path: Option<String>,
    #[command(subcommand)]
    pub commands: Command,
}

pub struct CliOptsCtx {
    pub custom_btf_path: Option<String>,
}

impl From<&CliOptions> for CliOptsCtx {
    fn from(value: &CliOptions) -> Self {
        Self {
            custom_btf_path: value.custom_btf_path.clone().into(),
        }
    }
}

#[derive(Subcommand)]
pub enum Command {
    #[command(subcommand)]
    Trace(trace::Subcommand),

    #[command(subcommand)]
    Perf(perf::Subcommand),
}

pub mod trace {
    use super::{Args, CliOptsCtx, Config};

    #[derive(Debug, clap::Subcommand)]
    pub enum Subcommand {
        #[command(name = "skbdrop")]
        SkbDrop(SkbDropOptions),
    }

    #[derive(Debug, Args)]
    pub struct SkbDropOptions {
        /// 是否支持v6
        #[arg(short = '6', long = "v6", default_value_t = false)]
        pub is_v6: bool,

        /// BPF packet filter expression using tcpdump/pcap syntax (e.g., 'tcp port 80')
        #[arg(short = 'f', long = "filter")]
        pub expression: Option<String>,

        /// Network interface to capture packets from (e.g., eth0, wlan0)
        #[arg(short = 'i', long = "iface")]
        pub interface: Option<String>,

        /// Process ID (PID) to filter traffic for a specific process
        #[arg(short = 'p', long = "pid", default_value_t = 0)]
        pub process_id: u32,

        /// Process name to filter traffic for matching processes
        #[arg(long = "pname")]
        pub process_name: Option<String>,

        /// Docker/Podman container ID to filter traffic for a specific container
        #[arg(long = "container-id")]
        pub container_id: Option<String>,

        /// Docker/Podman container name to filter traffic for matching containers
        #[arg(long = "container-name")]
        pub container_name: Option<String>,

        /// Kubernetes pod name to filter traffic for pods in a cluster
        #[arg(long = "pod")]
        pub pod_name: Option<String>,
    }

    impl SkbDropOptions {
        pub fn to_config(&self, ctx: CliOptsCtx) -> Config {
            Config {
                is_v6: self.is_v6,
                interface: self.interface.clone().unwrap_or_default(),
                process_id: self.process_id,
                process_name: self.process_name.clone().unwrap_or_default(),
                container_id: self.container_id.clone().unwrap_or_default(),
                container_name: self.container_name.clone().unwrap_or_default(),
                pod_name: self.pod_name.clone().unwrap_or_default(),
                filter_express: self.expression.clone().unwrap_or_default(),
                netns: 0,
                custom_btf_path: ctx.custom_btf_path,
            }
        }
    }
}

pub mod perf {
    use clap::ValueEnum;
    use clap::{Args, Parser};
    use opentrace_bpf::collector::cpu::ProfileConfig;

    use crate::errors::CliError;

    use super::CliOptsCtx;

    #[derive(Debug, clap::Subcommand)]
    pub enum Subcommand {
        #[command(name = "profile")]
        Profile(ProfileOptions),
    }

    #[derive(Clone, Copy, Debug, ValueEnum)]
    pub enum Language {
        Java,
    }

    #[derive(Debug, Args)]
    pub struct ProfileOptions {
        /// 针对指定pid采样, （不填写代表全采，0 代表只采集当前自己，>0
        /// 代表指定进程)
        #[arg(short = 'p', long = "pid",value_parser = clap::value_parser!(i32).range(1..))]
        pub pid: Option<i32>,

        /// 针对指定tid采样,必须提供pid (基于pid dumpsymbol)
        #[arg(long = "tid", requires = "pid")]
        pub tid: Option<u32>,

        /// 针对某一个cpu上所有进程都采样
        #[arg(short = 'c', long = "cpu", default_value_t = -1)]
        pub cpu: i32,

        /// 针对某一个cgroup里面的所有的进程都采样
        #[arg(short = 'g', long = "group", default_value_t = -1)]
        pub group_id: i32,

        /// 符号解析扩,支持针对指定类型的符号解析
        #[arg(long = "language")]
        pub language: Option<Language>,
    }

    impl ProfileOptions {
        pub fn to_config(self, ctx: CliOptsCtx) -> ProfileConfig {
            let pid: i32 = if let Some(pid) = self.pid { pid } else { -1 };
            ProfileConfig {
                pid: pid,
                tids: self.tid.map(|v| vec![v]),
                cpu: self.cpu,
                group_id: self.group_id,
                custom_btf_path: ctx.custom_btf_path,
            }
        }
    }
}
