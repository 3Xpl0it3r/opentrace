#ifndef OPENTRACE_BPF_PROCESS_H
#define OPENTRACE_BPF_PROCESS_H

#include "vmlinux.h"
#include "libbpf/src/bpf_helpers.h"

#define TASK_COMM_LEN 16

struct process_info_t {
  // 进程id
  u32 pid;
  // 线程id
  u32 tid;
  char comm[TASK_COMM_LEN];
};

static __always_inline void
set_process_info(struct process_info_t *process_info) {
  u64 tgid_pid = bpf_get_current_pid_tgid();
  bpf_get_current_comm(&process_info->comm, sizeof(process_info->comm));
  process_info->tid = (u32)tgid_pid;
  process_info->pid = tgid_pid >> 32;
}

static __always_inline bool filter_pid(u32 target_pid,
                                       struct process_info_t *proc_info) {
  // 如果没有指定targetpid则代表全抓
  if (target_pid == 0)
    return true;
  return target_pid == (proc_info->pid);
}

static __always_inline bool filter_comm(char *target_comm,
                                        struct process_info_t *proc_info) {
// 快速路径，如果高版本内核支持bpf字符串比较则使用bpfstrncmp来比较
#ifdef BPF_STRNCMP
  return bpf_strncmp(comm, TASK_COMM_LEN, target_comm) == 0;
#else
  // 慢路径，由于char comm[16]是128位，可以看成两个u64
  // ，直接用过位运算来加速对比
  return ((*(u64 *)target_comm ^ *(u64 *)proc_info->comm) |
          (*(u64 *)(target_comm + 8) ^ *(u64 *)(proc_info->comm + 8))) == 0;
#endif
}

#endif
