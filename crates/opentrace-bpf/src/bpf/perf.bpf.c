#include "vmlinux.h"

#include "libbpf/src/bpf_helpers.h"

#include "include/ebpf_map.h"

struct stack_key_t {
  u32 pid;
  int kernel_stack_id;
  int user_stack_id;
};

BPF_STACK_TRACE_DEF(stack_traces);

BPF_HASH_MAP_DEF(stack_count_map, sizeof(struct stack_key_t), sizeof(u64));

SEC("perf_event")
int perf_stack_samples(struct bpf_perf_event_data *ctx) {
  int kernel_stack_id = bpf_get_stackid(ctx, &stack_traces, 0);
  int user_stack_id = bpf_get_stackid(ctx, &stack_traces, BPF_F_USER_STACK);
  u32 pid = bpf_get_current_pid_tgid() >> 32;
  struct stack_key_t key = {
      .pid = pid,
      .kernel_stack_id = kernel_stack_id,
      .user_stack_id = user_stack_id,
  };
  u64 *count = bpf_map_lookup_elem(&stack_count_map, &key);
  if (count) {
    __sync_fetch_and_add(count, 1);
  } else {
    u64 init_val = 1;
    bpf_map_update_elem(&stack_count_map, &key, &init_val, BPF_NOEXIST);
  }
  return BPF_OK;
}

char _license[] SEC("license") = "GPL";
