// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.
use std::io::Read;
use std::path::{Path, PathBuf};

use crate::errors::EbpfError;

const KERNEL_BTF_PATH: &str = "/sys/kernel/btf/vmlinux";
const DEFAULT_CUSTOM_BTF_NAME: &str = "vmlinux.btf";
/// BTF 文件 magic（little-endian 写入磁盘后字节序为 0x9F 0xEB）。
const BTF_MAGIC: u16 = 0xeb9f;

pub fn setup_memlock_limit() {
    let rlim = libc::rlimit {
        rlim_cur: libc::RLIM_INFINITY,
        rlim_max: libc::RLIM_INFINITY,
    };
    let ret = unsafe { libc::setrlimit(libc::RLIMIT_MEMLOCK, &rlim) };
    if ret != 0 {
        eprintln!("Warning: Failed to remove memory lock limit (ret={})", ret);
    }
}

/// 检测当前环境是否具备可用的 BTF。
///
/// 判定流程：
/// 1. 内核原生支持（`/sys/kernel/btf/vmlinux` 可读）→ `Ok(())`
/// 2. 否则查找可执行文件同目录下的 `vmlinux.btf` 并校验合法性 → `Ok(())`
/// 3. 都不满足 → 返回 [`EbpfError::Other`]
pub fn check_btf_support() -> Result<bool, EbpfError> {
    if kernel_btf_available() {
        return Ok(true);
    }
    let fallback = default_custom_btf_path()?;
    validate_btf_file(&fallback).map_err(|e| {
        EbpfError::Other(format!(
            "kernel BTF not supported and no valid custom BTF at {}: {}",
            fallback.display(),
            e
        ))
    })?;
    Ok(false)
}

fn kernel_btf_available() -> bool {
    std::fs::File::open(KERNEL_BTF_PATH).is_ok()
}

pub fn default_custom_btf_path() -> Result<PathBuf, EbpfError> {
    let exe = std::env::current_exe()
        .map_err(|e| EbpfError::Other(format!("failed to resolve current exe: {}", e)))?;
    let dir = exe
        .parent()
        .ok_or_else(|| EbpfError::Other("current exe has no parent dir".to_string()))?;
    Ok(dir.join(DEFAULT_CUSTOM_BTF_NAME))
}

fn validate_btf_file(path: &Path) -> std::io::Result<()> {
    let mut file = std::fs::File::open(path)?;
    let mut magic = [0u8; 2];
    file.read_exact(&mut magic)?;
    let got = u16::from_le_bytes(magic);
    if got != BTF_MAGIC {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!(
                "invalid BTF magic: expected 0x{:04x}, got 0x{:04x}",
                BTF_MAGIC, got
            ),
        ));
    }
    Ok(())
}

/// 当前运行内核的版本号，编码为 (major, minor)。
///
/// 通过 `uname()` 解析 release 字段。failover：解析失败时返回 (0, 0)。
pub fn kernel_version() -> (u32, u32) {
    let mut buf: libc::utsname = unsafe { std::mem::zeroed() };
    if unsafe { libc::uname(&mut buf) } != 0 {
        return (0, 0);
    }
    let release = unsafe { std::ffi::CStr::from_ptr(buf.release.as_ptr()) };
    let s = match release.to_str() {
        Ok(s) => s,
        Err(_) => return (0, 0),
    };
    let mut it = s
        .split(|c: char| !c.is_ascii_digit())
        .filter(|p| !p.is_empty());
    let major = it.next().and_then(|s| s.parse().ok()).unwrap_or(0);
    let minor = it.next().and_then(|s| s.parse().ok()).unwrap_or(0);
    (major, minor)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn validates_btf_magic() {
        let file = tempfile::NamedTempFile::new().unwrap();
        fs::write(file.path(), BTF_MAGIC.to_le_bytes()).unwrap();

        assert!(validate_btf_file(file.path()).is_ok());
    }

    #[test]
    fn rejects_invalid_btf_magic() {
        let file = tempfile::NamedTempFile::new().unwrap();
        fs::write(file.path(), [0, 0]).unwrap();

        assert!(validate_btf_file(file.path()).is_err());
    }

    #[test]
    fn rejects_short_file() {
        let file = tempfile::NamedTempFile::new().unwrap();
        fs::write(file.path(), [0]).unwrap();

        assert!(validate_btf_file(file.path()).is_err());
    }

    #[test]
    fn rejects_nonexistent_file() {
        assert!(validate_btf_file(Path::new("/nonexistent/path")).is_err());
    }

    #[test]
    fn default_custom_btf_path_returns_path_with_vmlinux_btf_name() {
        let path = default_custom_btf_path().unwrap();
        assert_eq!(path.file_name().unwrap(), "vmlinux.btf");
    }

    #[test]
    fn kernel_version_returns_tuple() {
        let (major, minor) = kernel_version();
        // 在 Linux 上应该返回非零值，macOS 上可能返回 (0, 0)
        // 这里只验证函数不会 panic
        let _ = (major, minor);
    }
}
