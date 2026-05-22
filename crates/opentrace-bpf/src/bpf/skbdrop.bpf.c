#include "vmlinux.h"
#include "libbpf/src/bpf_endian.h"
#include "libbpf/src/bpf_helpers.h"
#include "libbpf/src/bpf_tracing.h"

#include "include/common.h"
#include "include/ebpf_map.h"
#include "include/net_filter.h"
#include "include/net_helper.h"
#include "include/net_types.h"
#include "include/process.h"

// ---------------------------------------------------------------------------
// 常量与 kconfig
// ---------------------------------------------------------------------------

#define PERF_MAX_STACK_DEPTH 16

// drop 事件来源标记，与用户态 DROP_SRC_* 对齐。
#define DROP_SRC_KFREE_SKB 1
#define DROP_SRC_NF_HOOK 2

extern int LINUX_KERNEL_VERSION __kconfig;

// ---------------------------------------------------------------------------
// 数据结构
// ---------------------------------------------------------------------------

// 用户态下发的过滤配置。
struct config {
  union addr any_addr;
  union addr src_addr;
  union addr dst_addr;
  u32 pid;
  u32 netns;
  u16 eth_proto;
  u16 ip_proto;
  u16 any_port;
  u16 src_port;
  u16 dst_port;
  u8 _pad[6];
};

// 上送用户态的事件。
struct perf_event_t {
  struct l2_info_t l2_info;
  struct l3_info_t l3_info;
  struct l4_info_t l4_info;
  s64 stack_size;
  u64 stack[PERF_MAX_STACK_DEPTH];
  u8 drop_reason;
  u8 drop_source;
};

// 套接字关联信息（保留供后续扩展使用）。
struct sock_owner_t {
  u64 pid_tgid;
  char comm[TASK_COMM_LEN];
  __be16 eth_proto;
  u16 ip_proto;
};

// kretprobe 上下文里调用栈已回退到 trampoline，必须在 entry 时抓栈暂存。
struct nf_hook_stack_t {
  s64 stack_size;
  u64 stack[PERF_MAX_STACK_DEPTH];
};

// ---------------------------------------------------------------------------
// Maps
// ---------------------------------------------------------------------------

BPF_HASH_MAP_DEF(config_map, u8, struct config);

// perf_event_t 较大，放 percpu array 避开 BPF 栈大小限制。
BPF_PERCPU_ARRAY_DEF(event_heap, struct perf_event_t, 1);

// per-CPU 计数器：>0 表示当前 CPU 正处于 nf_hook_slow 调用栈内，
// 用于在该路径上抑制 kfree_skb probe 重复上报。
BPF_PERCPU_ARRAY_DEF(in_nf_hook, u32, 1);

// per-CPU 暂存：nf_hook_slow 入口的 skb 指针，retprobe 时取出来用。
BPF_PERCPU_ARRAY_DEF(nf_hook_skb, u64, 1);

// per-CPU 暂存：nf_hook_slow 入口的调用栈，retprobe 时取出来覆盖事件栈。
BPF_PERCPU_ARRAY_DEF(nf_hook_stack, struct nf_hook_stack_t, 1);

BPF_PERF_EVENT_ARRAY_DEF(perf_events);

// ---------------------------------------------------------------------------
// 过滤 helpers
// ---------------------------------------------------------------------------

static __always_inline bool l2_filter(struct l2_info_t *l2,
                                      struct config *cfg) {
  // 发送路径 socket 包的 eth_proto 是 0，直接放行进行后续过滤。
  if (l2->eth_proto == 0)
    return true;
  return cfg->eth_proto == l2->eth_proto;
}

static __always_inline bool l3_filter(struct l3_info_t *l3, struct config *cfg,
                                      u8 ipvs) {
  if (!ipaddr_is_zero(cfg->any_addr))
    return filter_match_any_ip(cfg->any_addr, l3->saddr, l3->daddr, ipvs);

  if (!filter_match_exact_ip(cfg->dst_addr, l3->daddr, ipvs))
    return false;
  if (!filter_match_exact_ip(cfg->src_addr, l3->saddr, ipvs))
    return false;
  return true;
}

static __always_inline bool l4_filter(struct l4_info_t *l4,
                                      struct config *cfg) {
  u16 sport = bpf_ntohs(l4->sport);
  u16 dport = bpf_ntohs(l4->dport);

  if (cfg->any_port != 0)
    return cfg->any_port == sport || cfg->any_port == dport;

  if (cfg->dst_port != 0 && cfg->dst_port != dport)
    return false;
  if (cfg->src_port != 0 && cfg->src_port != sport)
    return false;
  return true;
}

// 解析 skb 的 L2/L3/L4 字段并过滤；命中过滤条件则填充 event 并返回 true。
static __always_inline bool do_trace_skbdrop(void *ctx, struct config *cfg,
                                             struct sk_buff *skb,
                                             struct perf_event_t *event) {
  set_l2_info(skb, &event->l2_info);

  unsigned char *network_header = skb_network_header(skb);
  u8 ipvs = ip_version(network_header);
  if (ipvs == 4)
    set_ipv4_info(skb, &event->l3_info);
  else if (ipvs == 6)
    set_ipv6_info(skb, &event->l3_info);
  else
    return false;

  if (!l3_filter(&event->l3_info, cfg, ipvs))
    return false;

  if (event->l3_info.ip_proto == IPPROTO_TCP)
    set_tcp_info(skb, &event->l4_info);
  else if (event->l3_info.ip_proto == IPPROTO_UDP)
    set_udp_info(skb, &event->l4_info);
  else
    return false;

  if (!l4_filter(&event->l4_info, cfg))
    return false;

  event->stack_size = bpf_get_stack(ctx, event->stack, sizeof(event->stack), 0);
  return true;
}

// nf_hook_slow 在不同内核中 skb 参数位置不同：
//   3.10 (CentOS 7):  nf_hook_slow(u8 pf, unsigned int hook, struct sk_buff
//   *skb, ...)
//                     → skb 是 PARM3
//   4.4+:             nf_hook_slow(struct sk_buff *skb, struct nf_hook_state
//   *state, ...)
//                     → skb 是 PARM1
//
// 必须用 volatile 强制 clang 各自独立 load 两个 ctx 字段，否则 clang 会
// 优化成 "select offset, ctx += offset, deref"，触发 verifier 的
// "dereference of modified ctx ptr" 错误。
static __always_inline struct sk_buff *
nf_hook_slow_skb_arg(struct pt_regs *ctx) {
  volatile unsigned long parm1 = PT_REGS_PARM1(ctx);
  volatile unsigned long parm3 = PT_REGS_PARM3(ctx);
  if (LINUX_KERNEL_VERSION >= KERNEL_VERSION(4, 4, 0))
    return (struct sk_buff *)parm1;
  return (struct sk_buff *)parm3;
}

// ---------------------------------------------------------------------------
// Programs
// ---------------------------------------------------------------------------

SEC("kprobe/kfree_skb")
int kp_kfree_skb(struct pt_regs *ctx) {
  // 当前 CPU 正处于 nf_hook_slow 调用栈内时跳过，避免与 kr_nf_hook_slow
  // 双计数。
  u32 k = 0;
  u32 *in_hook = bpf_map_lookup_elem(&in_nf_hook, &k);
  if (in_hook && *in_hook > 0)
    return BPF_OK;

  struct sk_buff *skb = (struct sk_buff *)PT_REGS_PARM1(ctx);
  u8 config_key = 0;
  u32 event_heap_key = 0;

  struct config *cfg = bpf_map_lookup_elem(&config_map, &config_key);
  if (!skb || !cfg)
    return BPF_OK;

  struct perf_event_t *event =
      bpf_map_lookup_elem(&event_heap, &event_heap_key);
  if (!event)
    return BPF_OK;

  event->drop_reason = 0;
  event->drop_source = DROP_SRC_KFREE_SKB;

  if (!do_trace_skbdrop(ctx, cfg, skb, event))
    return BPF_OK;

  bpf_perf_event_output(ctx, &perf_events, BPF_F_CURRENT_CPU, event,
                        sizeof(*event));
  return BPF_OK;
}

SEC("kprobe/nf_hook_slow")
int kp_nf_hook_slow(struct pt_regs *ctx) {
  u32 k = 0;

  // 递增 in_nf_hook 计数，告诉 kp_kfree_skb 跳过。
  u32 *cnt = bpf_map_lookup_elem(&in_nf_hook, &k);
  if (cnt) {
    u32 v = *cnt + 1;
    bpf_map_update_elem(&in_nf_hook, &k, &v, BPF_ANY);
  }

  // 暂存 skb 指针给 kretprobe 使用。
  struct sk_buff *skb = nf_hook_slow_skb_arg(ctx);
  u64 skb_ptr = (u64)skb;
  bpf_map_update_elem(&nf_hook_skb, &k, &skb_ptr, BPF_ANY);

  // entry 时调用栈完整，立刻抓栈暂存；retprobe 时栈已回退到 trampoline。
  struct nf_hook_stack_t *st = bpf_map_lookup_elem(&nf_hook_stack, &k);
  if (st)
    st->stack_size = bpf_get_stack(ctx, st->stack, sizeof(st->stack), 0);

  return BPF_OK;
}

SEC("kretprobe/nf_hook_slow")
int kret_nf_hook_slow(struct pt_regs *ctx) {
  u32 k = 0;
  int ret = (int)PT_REGS_RC(ctx);

  // 先无条件递减计数，确保后续 kfree_skb probe 正常工作。
  u32 *cnt = bpf_map_lookup_elem(&in_nf_hook, &k);
  if (cnt && *cnt > 0) {
    u32 v = *cnt - 1;
    bpf_map_update_elem(&in_nf_hook, &k, &v, BPF_ANY);
  }

  // netfilter verdict 语义（nf_hook_slow 返回值）：
  //   ret == 1 : NF_ACCEPT                            → 不关心
  //   ret <  0 : NF_DROP（含 iptables -j DROP/REJECT）→ 上报
  //   ret == 0 : NF_STOLEN / NF_QUEUE                 → 不算 drop，跳过
  if (ret >= 0)
    return BPF_OK;

  u64 *skb_p = bpf_map_lookup_elem(&nf_hook_skb, &k);
  if (!skb_p || *skb_p == 0)
    return BPF_OK;
  struct sk_buff *skb = (struct sk_buff *)(*skb_p);

  u8 config_key = 0;
  u32 event_heap_key = 0;
  struct config *cfg = bpf_map_lookup_elem(&config_map, &config_key);
  if (!cfg)
    return BPF_OK;

  struct perf_event_t *event =
      bpf_map_lookup_elem(&event_heap, &event_heap_key);
  if (!event)
    return BPF_OK;

  event->drop_reason = 0;
  event->drop_source = DROP_SRC_NF_HOOK;

  if (!do_trace_skbdrop(ctx, cfg, skb, event))
    return BPF_OK;

  // do_trace_skbdrop 在 retprobe 上下文抓的栈是 trampoline，
  // 用 entry 时暂存的真实调用栈覆盖。
  struct nf_hook_stack_t *st = bpf_map_lookup_elem(&nf_hook_stack, &k);
  if (st && st->stack_size > 0) {
    event->stack_size = st->stack_size;
    __builtin_memcpy(event->stack, st->stack, sizeof(event->stack));
  }

  bpf_perf_event_output(ctx, &perf_events, BPF_F_CURRENT_CPU, event,
                        sizeof(*event));
  return BPF_OK;
}

char _license[] SEC("license") = "GPL";
