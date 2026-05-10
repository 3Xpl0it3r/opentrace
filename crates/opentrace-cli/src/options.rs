// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.

use clap::{Parser, Subcommand};

use crate::config::Config;

#[derive(Parser)]
pub struct ServerOptions {}

#[derive(Parser)]
pub struct CliOptions {
    /// 是否支持v6
    #[arg(global = true, short = '6', long = "v6", default_value_t = false)]
    pub is_v6: bool,

    /// BPF packet filter expression using tcpdump/pcap syntax (e.g., 'tcp port 80')
    #[arg(global = true, short = 'f', long = "filter")]
    pub expression: Option<String>,

    /// Network interface to capture packets from (e.g., eth0, wlan0)
    #[arg(global = true, short = 'i', long = "iface")]
    pub interface: Option<String>,

    /// Process ID (PID) to filter traffic for a specific process
    #[arg(global = true, short = 'p', long = "pid", default_value_t = 0)]
    pub process_id: u32,

    /// Process name to filter traffic for matching processes
    #[arg(global = true, long = "pname")]
    pub process_name: Option<String>,

    /// Docker/Podman container ID to filter traffic for a specific container
    #[arg(global = true, long = "container-id")]
    pub container_id: Option<String>,

    /// "Docker/Podman container name to filter traffic for matching containers",
    #[arg(global = true, long = "container-name")]
    pub container_name: Option<String>,

    /// Kubernetes pod name to filter traffic for pods in a cluster
    #[arg(global = true, long = "pod")]
    pub pod_name: Option<String>,
    #[command(subcommand)]
    pub commands: Command,
}

impl CliOptions {
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

#[derive(Subcommand)]
pub enum Command {
    #[command(subcommand)]
    Trace(trace::Subcommand),
}

pub mod trace {
    #[derive(Debug, clap::Subcommand)]
    pub enum Subcommand {
        #[command(name = "skbdrop")]
        SkbDrop,
        /* Iptables(iptables::Command), */
    }
}
