#ifndef OPENTRACE_BPF_PROCESS_H
#define OPENTRACE_BPF_PROCESS_H

#include "vmlinux.h"
#include "libbpf/src/bpf_helpers.h"

#define TASK_COMM_LEN 16

struct process_info_t {
  u64 tgid_pid;
  char comm[TASK_COMM_LEN];
};

static __always_inline void set_process_info(struct process_info_t *process_info) {
  process_info->tgid_pid = bpf_get_current_pid_tgid();
  bpf_get_current_comm(&process_info->comm, sizeof(process_info->comm));
}

#endif
