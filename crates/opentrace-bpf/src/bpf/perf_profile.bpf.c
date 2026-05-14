#include "include/net_types.h"
#include "vmlinux.h"

#include "libbpf/src/bpf_helpers.h"

#include "include/ebpf_map.h"
#include "include/process.h"

#define PERF_MAX_STACK_DEPTH 16

// 配置文件
struct config {
  // 基于pid过滤
  u32 pid;
};

// 发送给用户态的event
struct perf_event_t {
  struct process_info_t process_info;
  u64 kstack[PERF_MAX_STACK_DEPTH];
  u64 ustack[PERF_MAX_STACK_DEPTH];
  s64 kstack_sz;
  s64 ustack_sz;
  u64 timestamp;
  u32 cpu_id;
};

// static const u8 config_key = 0;
static const u32 event_heap_key = 0;

// BPF_HASH_MAP_DEF(config_map, u8, struct config);

// BPF_STACK_TRACE_DEF(stack_traces);

BPF_PERF_EVENT_ARRAY_DEF(perf_events);

BPF_PERCPU_ARRAY_DEF(event_heap, struct perf_event_t, 1);

SEC("perf_event")
int perf_profile_samples(void *ctx) {
  struct perf_event_t *event = NULL;
  event = bpf_map_lookup_elem(&event_heap, &event_heap_key);
  if (!event)
    return BPF_OK;
  set_process_info(&event->process_info);
  event->kstack_sz =
      bpf_get_stack(ctx, event->kstack, sizeof(event->kstack), 0);
  event->ustack_sz = bpf_get_stack(ctx, event->ustack, sizeof(event->ustack_sz),
                                   BPF_F_USER_STACK);
  event->cpu_id = bpf_get_smp_processor_id();
  event->timestamp = bpf_ktime_get_ns();

  bpf_perf_event_output(ctx, &perf_events, BPF_F_CURRENT_CPU, event,
                        sizeof(*event));

  return BPF_OK;
}

char _license[] SEC("license") = "GPL";
