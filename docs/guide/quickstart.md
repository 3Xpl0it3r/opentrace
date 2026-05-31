# 快速开始

## 安装

### 1. Rust 工具链

```bash
rustup install 1.95.0
rustup default 1.95.0
```

### 2. 系统依赖

**Debian / Ubuntu：**

```bash
sudo apt update
sudo apt install -y clang llvm bpftool libelf-dev zlib1g-dev build-essential
```

**RHEL / CentOS / Fedora：**

```bash
sudo dnf install -y clang llvm bpftool elfutils-libelf-devel zlib-devel gcc make
```

### 3. BTF 准备

检查内核是否已暴露 BTF：

```bash
ls -l /sys/kernel/btf/vmlinux
```

**存在该文件**：无需额外操作，直接进入构建步骤。

**不存在该文件**：使用项目内置流程生成本地 BTF：

```bash
make install-pahole       # 安装 pahole (dwarves)
make install-debuginfo    # 安装当前内核 debuginfo / dbgsym
make vmlinux              # 生成 scripts/include/vmlinux.{h,btf}
```

> Arch Linux 官方仓库不提供 `kernel-debuginfo`，需从 AUR 安装 `linux-debug` 或自行编译带调试符号的内核。

### 4. 构建

推荐使用 Makefile（自动探测 BTF 并按需准备依赖）：

```bash
make build       # debug 构建
make release     # release 构建
make info        # 输出发行版 / 内核 / 架构 / BTF 检测结果
```

也可直接使用 cargo（要求 BTF 已就绪或已手动放置 `scripts/include/vmlinux.{h,btf}`）：

```bash
cargo build                              # 默认构建 opentrace-server
cargo build --package opentrace-cli      # 构建 CLI 工具
```

### 5. 可选：Java 符号支持

对 Java 进程进行性能剖析需要 [jallsyms](https://github.com/3Xpl0it3r/jallsyms)：

```bash
git clone https://github.com/3Xpl0it3r/jallsyms.git
cd jallsyms && make && sudo make install
```

## 验证安装

```bash
# 查看帮助
sudo ./target/debug/opentrace-cli --help

# 追踪 skb drop 事件
sudo ./target/debug/opentrace-cli trace skbdrop
```

## 下一步

- [CLI 命令](/guide/cli) - 了解 CLI 的详细用法
- [MCP 服务](/guide/mcp) - 了解 MCP 服务的使用
- [开发概览](/guide/development/overview) - 参与开发
