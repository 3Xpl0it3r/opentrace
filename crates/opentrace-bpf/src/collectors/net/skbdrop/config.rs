use std::{mem, slice};

use crate::EbpfError;
use crate::utils::net as net_utils;

// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.
// 暴露给用户态的配置文件, 转换成InnerConfig 再传给ebpf
#[derive(Default, Debug)]
pub struct Config {
    pub any_addr: String,
    pub src_addr: String,
    pub dst_addr: String,
    pub pid: u32,
    pub netns: u32,
    pub eth_proto: u16,
    pub ip_proto: u16,
    pub any_port: u16,
    pub src_port: u16,
    pub dst_port: u16,
    pub custom_btf_path: Option<String>,
}

// 传递给内核态 eBPF 的配置，字段与 bpf/skbdrop.c 里的 struct config 一一对应。
#[repr(C)]
pub(super) struct InnerConfig {
    any_addr: [u32; 4],
    src_addr: [u32; 4],
    dst_addr: [u32; 4],
    pid: u32,
    netns: u32,
    eth_proto: u16,
    ip_proto: u16,
    any_port: u16,
    src_port: u16,
    dst_port: u16,
    _pad: [u8; 6],
}

impl InnerConfig {
    pub(super) fn as_bytes(&self) -> &[u8] {
        let ptr = self as *const Self as *const u8;
        let len = mem::size_of_val(self);
        unsafe { slice::from_raw_parts(ptr, len) }
    }
}

impl Config {
    pub(super) fn into_inner(self) -> Result<InnerConfig, EbpfError> {
        Ok(InnerConfig {
            any_addr: net_utils::ipaddr_to_u128(&self.any_addr)?,
            src_addr: net_utils::ipaddr_to_u128(&self.src_addr)?,
            dst_addr: net_utils::ipaddr_to_u128(&self.dst_addr)?,
            pid: self.pid,
            netns: self.netns,
            eth_proto: self.eth_proto,
            ip_proto: self.ip_proto,
            any_port: self.any_port,
            src_port: self.src_port,
            dst_port: self.dst_port,
            _pad: [0; 6],
        })
    }
}
