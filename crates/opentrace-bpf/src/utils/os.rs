// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.

use std::collections::HashMap;
use std::ffi::CStr;
use std::fs::{self, File};
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::sync::LazyLock;

use serde::ser::SerializeSeq;
use serde::Serialize;

use crate::EbpfError;

const BTF_VMLINUX_PATH: &str = "/sys/kernel/btf/vmlinux";
const KALLSYMS_PATH: &str = "/proc/kallsyms";
const OS_RELEASE_PATH: &str = "/etc/os-release";
const PROC_VERSION_PATH: &str = "/proc/version";

static KALLSYMS: LazyLock<Vec<(u64, Box<str>)>> = LazyLock::new(|| read_kallsyms());

pub struct SystemInfo {
    pub id: String,
    pub version_id: String,
    pub kernel_version: String,
    pub arch: String,
}

pub struct SymbolizedStack<'a>(pub &'a [u64]);

struct SymbolWithOffset<'a>(&'a str, u64);

impl SystemInfo {
    pub fn try_parse() -> Result<Self, EbpfError> {
        let os_release = read_os_release().unwrap_or_default();
        let (kernel_version, arch) = kernel_info()?;

        Ok(SystemInfo {
            id: os_release
                .get("ID")
                .cloned()
                .unwrap_or_else(|| "UnKnown OS".to_string()),
            version_id: os_release
                .get("VERSION_ID")
                .cloned()
                .unwrap_or_else(|| "Unknown Os Version".to_string()),
            kernel_version,
            arch,
        })
    }
}

impl Serialize for SymbolizedStack<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut seq = serializer.serialize_seq(Some(self.0.len()))?;
        for addr in self.0 {
            if let Some((sym, offset)) = kallsyms_by_addr(addr) {
                seq.serialize_element(&SymbolWithOffset(sym, offset))?;
            } else {
                seq.serialize_element(&Option::<()>::None)?;
            }
        }
        seq.end()
    }
}

impl Serialize for SymbolWithOffset<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.collect_str(&format_args!("{}({:#x})", self.0, self.1))
    }
}

pub fn has_btf_support() -> bool {
    Path::new(BTF_VMLINUX_PATH).exists()
}

#[inline]
pub fn kallsyms_by_addr(addr: &u64) -> Option<(&'static str, u64)> {
    let sorted: &Vec<(u64, Box<str>)> = KALLSYMS.as_ref();

    match sorted.binary_search_by_key(addr, |(k, _)| *k) {
        Ok(idx) => Some((sorted[idx].1.as_ref(), 0)),
        Err(idx) if idx > 0 => {
            let (sym_addr, sym_name) = &sorted[idx - 1];
            Some((sym_name.as_ref(), addr - *sym_addr))
        }
        _ => None,
    }
}

#[inline]
pub fn cstr_to_string(data: &[u8]) -> String {
    match CStr::from_bytes_until_nul(data) {
        Ok(device_name) => device_name.to_string_lossy().to_string(),
        Err(_) => "".to_string(),
    }
}

#[inline]
fn read_os_release() -> Result<HashMap<String, String>, EbpfError> {
    let content = match fs::read_to_string(OS_RELEASE_PATH) {
        Ok(content) => content,
        Err(_) => return Ok(HashMap::new()),
    };

    Ok(content
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .filter_map(|line| line.split_once('='))
        .map(|(key, value)| (key.to_string(), value.trim_matches('"').to_string()))
        .collect())
}

#[inline]
fn read_kallsyms() -> Vec<(u64, Box<str>)> {
    let file = match File::open(KALLSYMS_PATH) {
        Ok(f) => f,
        Err(_) => return Vec::new(),
    };
    let reader = BufReader::new(file);

    let mut symbol_vec: Vec<(u64, Box<str>)> = Vec::new();
    for line in reader.lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => continue,
        };
        let mut parts = line.split_whitespace();
        let addr = match parts.next().and_then(|s| u64::from_str_radix(s, 16).ok()) {
            Some(addr) => addr,
            None => continue,
        };
        let _ = parts.next();
        if let Some(symbol) = parts.next() {
            symbol_vec.push((addr, symbol.to_owned().into_boxed_str()));
        }
    }
    symbol_vec.sort_by_key(|(k, _)| *k);
    symbol_vec
}

#[inline]
fn kernel_info() -> Result<(String, String), EbpfError> {
    // 读取内核版本信息
    let content = fs::read_to_string(PROC_VERSION_PATH).map_err(EbpfError::IO)?;

    // 提取版本号的第二个单词（示例: "Linux version 5.15.0-... ..."）
    let full_vsn = content.split_whitespace().nth(2).unwrap_or("").to_owned();

    // 从版本字符串中提取CPU架构（最后一个以'.'分割的部分）
    let arch = full_vsn.split('.').last().unwrap_or_default().to_string();

    Ok((full_vsn, arch))
}
