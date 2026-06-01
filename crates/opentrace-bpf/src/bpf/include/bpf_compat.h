#ifndef OPENTRACE_BPF_BTF_COMPAT_H
#define OPENTRACE_BPF_BTF_COMPAT_H
// 4.19内核里面测试发现vmlinux并没有这个变量定义导致报错
#ifndef BPF_F_CURRENT_CPU
#define BPF_F_CURRENT_CPU 0xFFFFFFFFULL
#endif

// 在bclinux(4.19)内核上编译报错 'BPF_F_USER_STACK'; did you mean
// 'TRACE_USER_STACK'? 在这个地方手动添加
#ifndef BPF_F_USER_STACK
#define BPF_F_USER_STACK (1ULL << 8)
#endif

#ifndef BPF_OK
#define BPF_OK 0
#endif

#ifndef BPF_DROP
#define BPF_DROP 2
#endif

#ifndef BPF_ANY
#define BPF_ANY 0
#endif

#ifndef BPF_NOEXIST
#define BPF_NOEXIST 1
#endif

#ifndef BPF_EXIST
#define BPF_EXIST 2
#endif

#ifndef BPF_F_LOCK
#define BPF_F_LOCK 4
#endif

#ifndef BPF_F_FAST_STACK_CMP
#define BPF_F_FAST_STACK_CMP (1ULL << 9)
#endif
#ifndef BPF_F_REUSE_STACKID
#define BPF_F_REUSE_STACKID (1ULL << 10)
#endif

#endif
