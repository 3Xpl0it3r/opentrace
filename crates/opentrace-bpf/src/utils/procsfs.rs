// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use crate::EbpfError;

const BTF_VMLINUX_PATH: &str = "/sys/kernel/btf/vmlinux";
const OS_RELEASE_PATH: &str = "/etc/os-release";
const PROC_VERSION_PATH: &str = "/proc/version";

pub struct SystemInfo {
    pub id: String,
    pub version_id: String,
    pub kernel_version: String,
    pub arch: String,
}

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

pub fn has_btf_support() -> bool {
    Path::new(BTF_VMLINUX_PATH).exists()
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
fn kernel_info() -> Result<(String, String), EbpfError> {
    // 读取内核版本信息
    let content = fs::read_to_string(PROC_VERSION_PATH).map_err(EbpfError::IO)?;

    // 提取版本号的第二个单词（示例: "Linux version 5.15.0-... ..."）
    let full_vsn = content.split_whitespace().nth(2).unwrap_or("").to_owned();

    // 从版本字符串中提取CPU架构（最后一个以'.'分割的部分）
    let arch = full_vsn
        .split('.')
        .next_back()
        .unwrap_or_default()
        .to_string();

    Ok((full_vsn, arch))
}

// 根据进程pid ，获取该进程下所有的线程
pub fn thread_ids(pid: u32) -> Result<Vec<u32>, EbpfError> {
    let task_dir = format!("/proc/{}/task", pid);
    let entries = fs::read_dir(task_dir)?;

    let mut thread_ids = Vec::new();
    for entry in entries {
        let entry = entry?;
        // 目录名即为线程 ID (TID)
        let tid_str = entry.file_name();
        if let Ok(tid) = tid_str.to_string_lossy().parse::<u32>() {
            thread_ids.push(tid);
        }
    }
    Ok(thread_ids)
}
