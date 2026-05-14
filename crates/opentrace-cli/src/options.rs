// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.

use clap::{Args, Parser, Subcommand};

use crate::config::Config;

#[derive(Parser)]
pub struct ServerOptions {}

#[derive(Parser)]
pub struct CliOptions {
    #[command(subcommand)]
    pub commands: Command,
}

#[derive(Subcommand)]
pub enum Command {
    #[command(subcommand)]
    Trace(trace::Subcommand),

    #[command(subcommand)]
    Perf(perf::Subcommand),
}

pub mod trace {
    use super::{Args, Config};

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
        pub fn to_config(&self) -> Config {
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
            }
        }
    }
}

pub mod perf {
    use clap::Args;
    use opentrace_bpf::collector::cpu::ProfileConfig;

    #[derive(Debug, clap::Subcommand)]
    pub enum Subcommand {
        #[command(name = "profile")]
        Profile(ProfileOptions),
    }

    #[derive(Debug, Args)]
    pub struct ProfileOptions {
        /// Process ID (PID) to profile. 0 means the current process.
        #[arg(short = 'p', long = "pid", default_value_t = 0)]
        pub pid: i32,

        /// CPU to profile. -1 means all CPUs supported by the perf event builder.
        #[arg(short = 'c', long = "cpu", default_value_t = -1)]
        pub cpu: i32,

        /// Cgroup id
        #[arg(short = 'g', long = "group", default_value_t = -1)]
        pub group_id: i32,
    }

    impl From<ProfileOptions> for ProfileConfig {
        fn from(options: ProfileOptions) -> Self {
            Self {
                pid: options.pid,
                cpu: options.cpu,
                group_id: options.group_id,
            }
        }
    }
}
