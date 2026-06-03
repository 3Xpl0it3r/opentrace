# opentrace-bpf Fuzz 测试与内存测试

本目录包含 `opentrace-bpf` crate 的 fuzz 测试和内存安全测试。

## 目录结构

```
crates/opentrace-bpf/
├── fuzz/                          # Fuzz 测试目录
│   ├── Cargo.toml                 # Fuzz 测试依赖配置
│   └── fuzz_targets/              # Fuzz 测试目标
│       ├── fuzz_http_parser.rs    # HTTP 解析器 fuzz
│       ├── fuzz_http1_request.rs  # HTTP/1.x 请求 fuzz
│       ├── fuzz_http2_frame.rs    # HTTP/2 帧 fuzz
│       ├── fuzz_hpack_decode.rs   # HPACK 解码 fuzz
│       ├── fuzz_protocol_parse.rs # 协议解析 fuzz
│       └── fuzz_types_serialize.rs # 类型序列化 fuzz
└── scripts/                       # 测试脚本
    ├── miri-test.sh               # MIRI 测试
    ├── asan-test.sh               # AddressSanitizer 测试
    └── memory-test.sh             # 综合内存测试
```

## Fuzz 测试

### 前置条件

安装 cargo-fuzz：
```bash
cargo install cargo-fuzz
```

### 运行 Fuzz 测试

```bash
cd crates/opentrace-bpf

# 运行 HTTP 解析器 fuzz
cargo fuzz run fuzz_http_parser

# 运行 HTTP/1.x 请求 fuzz
cargo fuzz run fuzz_http1_request

# 运行 HTTP/2 帧 fuzz
cargo fuzz run fuzz_http2_frame

# 运行 HPACK 解码 fuzz
cargo fuzz run fuzz_hpack_decode

# 运行协议解析 fuzz
cargo fuzz run fuzz_protocol_parse

# 运行类型序列化 fuzz
cargo fuzz run fuzz_types_serialize
```

### 自定义 Fuzz 参数

```bash
# 限制运行时间（秒）
cargo fuzz run fuzz_http_parser -- -max_total_time=60

# 限制内存（MB）
cargo fuzz run fuzz_http_parser -- -rss_limit_mb=2048

# 使用多个 worker
cargo fuzz run fuzz_http_parser -- -workers=4

# 运行特定种子
cargo fuzz run fuzz_http_parser corpus/http_parser/
```

### Fuzz 测试覆盖范围

| Fuzz Target | 测试目标 | 检测能力 |
|-------------|---------|---------|
| `fuzz_http_parser` | HttpParser::parse, hash_id | 解析崩溃、panic |
| `fuzz_http1_request` | HTTP/1.x 请求解析 | 格式处理、边界条件 |
| `fuzz_http2_frame` | HTTP/2 帧解析 | 帧格式、preface 处理 |
| `fuzz_hpack_decode` | HPACK 头部解码 | 压缩/解压正确性 |
| `fuzz_protocol_parse` | eth_proto, ip_proto 解析 | 协议名称解析 |
| `fuzz_types_serialize` | Addr, L2/L3/L4Info 序列化 | 序列化正确性 |

## 内存测试

### MIRI 测试

MIRI 是 Rust 的内存安全检查工具，可以检测：
- 未定义行为 (UB)
- 内存越界访问
- 使用未初始化内存
- 数据竞争

```bash
# 运行所有 MIRI 测试
./scripts/memory-test.sh miri

# 或直接运行
./scripts/miri-test.sh
```

### AddressSanitizer (ASAN)

ASAN 可以检测：
- 堆缓冲区溢出
- 栈缓冲区溢出
- 使用释放后的内存
- 内存泄漏

```bash
# 运行 ASAN 测试
./scripts/memory-test.sh asan

# 或直接运行
./scripts/asan-test.sh
```

### Valgrind

Valgrind 可以检测：
- 内存泄漏
- 未初始化内存使用
- 无效内存访问

```bash
# 运行 Valgrind 测试
./scripts/memory-test.sh valgrind
```

### 运行所有内存测试

```bash
./scripts/memory-test.sh all
```

## 测试目标说明

### 可测试的模块

| 模块 | 可测试性 | 说明 |
|------|---------|------|
| `protocols/http.rs` | ✅ 高 | 纯解析逻辑，无外部依赖 |
| `protocols/ether.rs` | ✅ 高 | 纯查表函数 |
| `protocols/inet.rs` | ✅ 高 | 纯查表函数 |
| `types/net.rs` | ✅ 高 | 类型定义和序列化 |
| `utils/net.rs` | ✅ 高 | 网络工具函数 |
| `utils/cstring.rs` | ✅ 高 | C 字符串转换 |
| `utils/time.rs` | ✅ 高 | 时间格式化 |
| `utils/bytes.rs` | ✅ 高 | 字节单位格式化 |

### 不可直接测试的模块

| 模块 | 原因 | 替代方案 |
|------|------|---------|
| `collectors/*/collector.rs` | 依赖 eBPF skeleton | 集成测试 |
| `symbolizers/balzesym.rs` | 依赖 blazesym 库 | Mock 测试 |
| `symbolizers/java.rs::new()` | 依赖外部命令 jallsyms | 解析逻辑已分离测试 |
| `utils/procfs.rs` | 依赖 /proc 文件系统 | Mock 文件系统 |
| `utils/syscall.rs::build()` | 依赖 perf_event 系统调用 | 配置逻辑可测 |

## 最佳实践

1. **定期运行 fuzz 测试**：建议在 CI 中定期运行 fuzz 测试
2. **保存崩溃用例**：fuzz 发现的崩溃用例应保存到语料库
3. **增量测试**：新功能添加后应更新 fuzz 测试
4. **内存测试覆盖**：关键路径应同时使用 MIRI 和 ASAN 测试

## 相关链接

- [cargo-fuzz 文档](https://rust-fuzz.github.io/book/)
- [MIRI 文档](https://github.com/rust-lang/miri)
- [AddressSanitizer](https://github.com/google/sanitizers/wiki/AddressSanitizer)
- [Valgrind](https://valgrind.org/)
