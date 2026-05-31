# 环境要求

## 系统要求

| 要求 | 最低版本 | 说明 |
|------|---------|------|
| Linux 内核 | ≥ 3.10 | 需要支持 kprobe / eBPF |
| Rust | ≥ 1.95.0 | 用于编译项目 |
| 权限 | root | 运行时需 root 或等价的 BPF / 内核追踪权限 |

## 内核特性支持

| 内核版本 | 支持的特性 |
|---------|-----------|
| 3.10+ | kprobe, kretprobe |
| 4.1+ | tracepoint |
| 4.9+ | eBPF maps, perf event |
| 5.2+ | BTF (BPF Type Format) |
| 5.16+ | kfree_skb_reason |

## BTF 支持

BTF (BPF Type Format) 是现代 eBPF 程序的重要特性，可以简化内核数据结构的访问。

### 检查 BTF 支持

```bash
# 检查内核是否暴露 BTF
ls -l /sys/kernel/btf/vmlinux

# 检查内核配置
grep CONFIG_DEBUG_INFO_BTF /boot/config-$(uname -r)
```

### 没有 BTF 支持

如果你的内核不支持 BTF，可以使用项目内置流程生成本地 BTF：

```bash
make install-pahole       # 安装 pahole (dwarves)
make install-debuginfo    # 安装当前内核 debuginfo / dbgsym
make vmlinux              # 生成 scripts/include/vmlinux.{h,btf}
```

## 系统依赖

### Debian / Ubuntu

```bash
sudo apt update
sudo apt install -y clang llvm bpftool libelf-dev zlib1g-dev build-essential
```

### RHEL / CentOS / Fedora

```bash
sudo dnf install -y clang llvm bpftool elfutils-libelf-devel zlib-devel gcc make
```

### Arch Linux

```bash
sudo pacman -S clang llvm bpftool libelf zlib
```

注意：Arch Linux 官方仓库不提供 `kernel-debuginfo`，需从 AUR 安装 `linux-debug` 或自行编译带调试符号的内核。

## Rust 工具链

```bash
# 安装 Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# 安装指定版本
rustup install 1.95.0
rustup default 1.95.0

# 验证安装
rustc --version
cargo --version
```
