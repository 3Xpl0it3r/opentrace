use std::fs;

use crate::EbpfError;

// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.
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
