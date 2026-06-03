// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.

use std::fmt::{self, Display, Formatter};

use opentrace_bpf::collectors::net::SkbdropConfig;
use opentrace_bpf::protocols::{eth_proto, ip_proto};

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
    pub custom_btf_path: Option<String>,
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
            custom_btf_path: config.custom_btf_path,
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

// filter语法树解析AI辅助写的

fn parse_filter_to_config(expr: String) -> Result<FilterConfig, CliError> {
    let mut cfg = FilterConfig::default();
    if expr.trim().is_empty() {
        return Ok(cfg);
    }

    let mut flags = ParseFlags::default();
    let mut tokens = expr.split_whitespace();
    while let Some(tok) = tokens.next() {
        apply_token(tok, &mut tokens, &mut cfg, &mut flags);
    }

    apply_defaults(&mut cfg, &flags)?;
    Ok(cfg)
}

/// 单个 token 的分发：根据 keyword 决定如何更新 cfg / flags，
/// 必要时从 `tokens` 取后续参数（host/port 等）。未识别的 token 静默跳过。
fn apply_token<'a, I: Iterator<Item = &'a str>>(
    tok: &str,
    tokens: &mut I,
    cfg: &mut FilterConfig,
    flags: &mut ParseFlags,
) {
    match tok {
        "and" => {}
        "tcp" => {
            cfg.ip_proto = ip_proto::TCP;
            flags.has_tcp_udp = true;
        }
        "udp" => {
            cfg.ip_proto = ip_proto::UDP;
            flags.has_tcp_udp = true;
        }
        "icmp" => {
            cfg.ip_proto = ip_proto::ICMP;
            flags.has_icmp = true;
        }
        "host" => take_ip(tokens, &mut cfg.any_ip),
        "port" => {
            take_port(tokens, &mut cfg.any_port);
            flags.has_port = true;
        }
        "src" => parse_direction(
            tokens,
            &mut cfg.src_ip,
            &mut cfg.src_port,
            &mut flags.has_port,
        ),
        "dst" => parse_direction(
            tokens,
            &mut cfg.dst_ip,
            &mut cfg.dst_port,
            &mut flags.has_port,
        ),
        _ => {}
    }
}

#[derive(Default)]
struct ParseFlags {
    has_tcp_udp: bool,
    has_port: bool,
    has_icmp: bool,
}

fn take_ip<'a, I: Iterator<Item = &'a str>>(it: &mut I, slot: &mut String) {
    if let Some(v) = it.next() {
        *slot = v.into();
    }
}

fn take_port<'a, I: Iterator<Item = &'a str>>(it: &mut I, slot: &mut u16) {
    if let Some(v) = it.next()
        && let Ok(p) = v.parse::<u16>()
    {
        *slot = p;
    }
}

/// 解析 `src host X` / `src port N` / `dst host X` / `dst port N`。
fn parse_direction<'a, I: Iterator<Item = &'a str>>(
    it: &mut I,
    ip_slot: &mut String,
    port_slot: &mut u16,
    has_port: &mut bool,
) {
    match it.next() {
        Some("host") => take_ip(it, ip_slot),
        Some("port") => {
            take_port(it, port_slot);
            *has_port = true;
        }
        _ => {}
    }
}

/// 解析阶段标志位经规范化后的语义状态。
enum ParseOutcome {
    /// 既无协议也无端口：保持默认值。
    Empty,
    /// icmp + port：非法组合。
    IcmpWithPort,
    /// tcp/udp 但没指定 port：非法组合。
    TcpUdpWithoutPort,
    /// 显式指定了 tcp/udp + port，或仅 icmp：协议已明确。
    ExplicitProto,
    /// 只给了 port 没给协议：默认按 tcp。
    PortOnly,
}

impl ParseFlags {
    fn classify(&self) -> ParseOutcome {
        match (self.has_icmp, self.has_tcp_udp, self.has_port) {
            (false, false, false) => ParseOutcome::Empty,
            (true, _, true) => ParseOutcome::IcmpWithPort,
            (_, true, false) => ParseOutcome::TcpUdpWithoutPort,
            (false, false, true) => ParseOutcome::PortOnly,
            _ => ParseOutcome::ExplicitProto,
        }
    }
}

fn apply_defaults(cfg: &mut FilterConfig, flags: &ParseFlags) -> Result<(), CliError> {
    match flags.classify() {
        ParseOutcome::Empty => Ok(()),
        ParseOutcome::IcmpWithPort => Err(CliError::ArgsErr(
            "icmp protocol cannot be used with port specification".into(),
        )),
        ParseOutcome::TcpUdpWithoutPort => Err(CliError::ArgsErr(
            "tcp/udp protocol requires a port specification".into(),
        )),
        ParseOutcome::PortOnly => {
            cfg.ip_proto = ip_proto::parse("tcp")?;
            cfg.eth_proto = eth_proto::ETH_P_IP;
            Ok(())
        }
        ParseOutcome::ExplicitProto => {
            cfg.eth_proto = eth_proto::ETH_P_IP;
            Ok(())
        }
    }
}
