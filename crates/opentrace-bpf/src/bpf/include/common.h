#ifndef OPENTRACE_BPF_COMMON_H
#define OPENTRACE_BPF_COMMON_H

#include "libbpf/src/bpf_helpers.h"

// 4.19内核里面测试发现vmlinux并没有这个变量定义导致报错
#ifndef BPF_F_CURRENT_CPU
#define BPF_F_CURRENT_CPU 0xFFFFFFFFULL
#endif

#ifndef KERNEL_VERSION
#define KERNEL_VERSION(a, b, c) (((a) << 16) + ((b) << 8) + (c))
#endif

/*
 * 从 tracepoint 原始上下文中按索引读取系统调用参数。
 */
#define TP_SYS_REGS_PARAM(dst, idx, ctx)                                       \
  bpf_probe_read_kernel(&dst, sizeof(dst),                                     \
                        (void *)ctx + sizeof(struct trace_entry) +             \
                            sizeof(long int) +                                 \
                            idx * (sizeof(long unsigned int)));

/*
 * 从 tracepoint 原始上下文中读取系统调用返回值。
 */
#define TP_SYS_REGS_RET(dst, ctx)                                              \
  bpf_probe_read_kernel(&dst, sizeof(dst),                                     \
                        (void *)ctx + sizeof(struct trace_entry) +             \
                            sizeof(long int));

/*
 * 获取当前 kprobe/kretprobe 的函数指令地址。
 */
#define KP_FUNC_IP(ctx)                                                        \
  ({                                                                           \
    u64 __ip;                                                                  \
    if (LINUX_KERNEL_VERSION >= KERNEL_VERSION(5, 15, 0)) {                    \
      __ip = bpf_get_func_ip(ctx);                                             \
    } else {                                                                   \
      /* x86架构需要-1字节修正 */                                              \
      __ip = PT_REGS_IP((struct pt_regs *)(ctx)) - 1;                          \
    }                                                                          \
    __ip;                                                                      \
  })

static __always_inline void my_memcpy(void *dst, const void *src,
                                      unsigned long size) {
  __builtin_memcpy(dst, src, size);
}

// 用来清零操作
static __always_inline void my_memset(void *dst, unsigned long size) {
  __builtin_memset(dst, 0, size);
}

#endif
