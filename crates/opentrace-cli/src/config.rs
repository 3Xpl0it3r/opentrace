// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.

use std::fmt::{self, Display, Formatter};

use opentrace_bpf::consts::{eth_proto, ip_proto};
use opentrace_bpf::prog::net::SkbdropConfig;

use crate::errors::CliError;

#[derive(Debug, Clone, Default)]
pub struct Config {
    pub is_v6: bool,
    pub interface: String,
    pub process_id: u32,
    pub process_name: String,
    pub container_id: String,
    pub container_name: String,
    pub pod_name: String,
    pub netns: u32,
    //filter_express是一个简易的bpf表达式
    pub filter_express: String,
}

impl Config {
    pub fn validate(&self) -> Result<(), CliError> {
        match parse_filter_to_config(self.filter_express.clone()) {
            Ok(cfg) => {
                println!("{}", cfg);
                Ok(())
            }
            Err(e) => Err(e),
        }
    }
}

// tracer/net/skbdrop/config 配置文件
impl From<Config> for SkbdropConfig {
    fn from(config: Config) -> Self {
        let fcfg = parse_filter_to_config(config.filter_express).unwrap();
        SkbdropConfig {
            any_addr: fcfg.any_ip,
            src_addr: fcfg.src_ip,
            dst_addr: fcfg.dst_ip,
            pid: config.process_id,
            netns: 0,
            eth_proto: fcfg.eth_proto,
            ip_proto: fcfg.ip_proto,
            any_port: fcfg.any_port,
            src_port: fcfg.src_port,
            dst_port: fcfg.dst_port,
            /* ip_version: if config.is_v6 { 6 } else { 4 }, */
        }
    }
}

#[derive(Default)]
#[repr(C)]
struct FilterConfig {
    ip_proto: u16,
    eth_proto: u16,
    any_port: u16,
    src_port: u16,
    dst_port: u16,
    any_ip: String,
    src_ip: String,
    dst_ip: String,
}

impl Display for FilterConfig {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        writeln!(f, "╔══════════════════════════════════════╗")?;
        writeln!(f, "║ {:36} ║", "FilterConfig")?;
        writeln!(f, "╠══════════════════════════════════════╣")?;
        writeln!(
            f,
            "║ {:<7}: {:<27} ║",
            "proto",
            ip_proto::to_str(self.ip_proto)
        )?;
        writeln!(
            f,
            "║ {:<7}: {:<27} ║",
            "family",
            eth_proto::to_str(self.eth_proto)
        )?;
        writeln!(
            f,
            "║ {:<7}: {:<27} ║",
            "any",
            format!(
                "{}:{}",
                self.any_ip,
                if self.any_port != 0 {
                    self.any_port
                } else if self.dst_port != 0 {
                    self.dst_port
                } else {
                    self.src_port
                }
            )
        )?;
        writeln!(
            f,
            "║ {:<7}: {:<27} ║",
            "src",
            format!(
                "{}:{}",
                self.src_ip,
                if self.src_port != 0 {
                    self.src_port
                } else {
                    self.any_port
                }
            )
        )?;
        writeln!(
            f,
            "║ {:<7}: {:<27} ║",
            "dst",
            format!(
                "{}:{}",
                self.dst_ip,
                if self.dst_port != 0 {
                    self.dst_port
                } else {
                    self.any_port
                }
            )
        )?;
        write!(f, "╚══════════════════════════════════════╝")
    }
}

fn parse_filter_to_config(expr: String) -> Result<FilterConfig, CliError> {
    let mut config = FilterConfig::default();

    if expr.trim().is_empty() {
        return Ok(config);
    }

    let tokens: Vec<&str> = expr.split_whitespace().collect();
    let mut i = 0;
    let mut has_tcp_udp = false;
    let mut has_port = false;
    let mut has_icmp = false;

    while i < tokens.len() {
        let token = tokens[i];

        match token {
            "and" => {
                i += 1;
                continue;
            }
            "tcp" => {
                config.ip_proto = ip_proto::IPPROTO_TCP;
                has_tcp_udp = true;
                i += 1;
            }
            "udp" => {
                config.ip_proto = ip_proto::IPPROTO_UDP;
                has_tcp_udp = true;
                i += 1;
            }
            "icmp" => {
                config.ip_proto = ip_proto::IPPROTO_ICMP;
                has_icmp = true;
                i += 1;
            }
            "host" => {
                if i + 1 < tokens.len() {
                    config.any_ip = tokens[i + 1].into();
                    i += 2;
                } else {
                    i += 1;
                }
            }
            "src" => {
                if i + 1 < tokens.len() {
                    match tokens[i + 1] {
                        "host" => {
                            if i + 2 < tokens.len() {
                                config.src_ip = tokens[i + 2].into();
                                i += 3;
                            } else {
                                i += 2;
                            }
                        }
                        "port" => {
                            if i + 2 < tokens.len() {
                                let port_str = tokens[i + 2];
                                if let Ok(port) = port_str.parse::<u16>() {
                                    config.src_port = port;
                                }
                                has_port = true;
                                i += 3;
                            } else {
                                i += 2;
                            }
                        }
                        _ => i += 1,
                    }
                } else {
                    i += 1;
                }
            }
            "dst" => {
                if i + 1 < tokens.len() {
                    match tokens[i + 1] {
                        "host" => {
                            if i + 2 < tokens.len() {
                                config.dst_ip = tokens[i + 2].into();
                                i += 3;
                            } else {
                                i += 2;
                            }
                        }
                        "port" => {
                            if i + 2 < tokens.len() {
                                let port_str = tokens[i + 2];
                                if let Ok(port) = port_str.parse::<u16>() {
                                    config.dst_port = port;
                                }
                                has_port = true;
                                i += 3;
                            } else {
                                i += 2;
                            }
                        }
                        _ => i += 1,
                    }
                } else {
                    i += 1;
                }
            }
            "port" => {
                if i + 1 < tokens.len() {
                    let port_str = tokens[i + 1];
                    if let Ok(port) = port_str.parse::<u16>() {
                        config.any_port = port;
                    }
                    has_port = true;
                    i += 2;
                } else {
                    i += 1;
                }
            }
            _ => i += 1,
        }
    }

    // 指定了port，但是没有指定具体协议(tcp/udp)，那么默认则是tcp
    if has_port && !has_tcp_udp && !has_icmp {
        config.ip_proto = ip_proto::parse("tcp")?;
    }

    if has_icmp && has_port {
        return Err(CliError::ArgsErr(
            "icmp protocol cannot be used with port specification".into(),
        ));
    }

    if has_tcp_udp && !has_port {
        return Err(CliError::ArgsErr(
            "tcp/udp protocol requires a port specification".into(),
        ));
    }

    // 如果指定了协议 或者 指定icmp协议 或者指定端口, 则默认使用ip v4协议
    if has_tcp_udp || has_icmp || has_port {
        config.eth_proto = eth_proto::ETH_P_IP;
    }

    Ok(config)
}
