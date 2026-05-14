#include "vmlinux.h"
#include "libbpf/src/bpf_endian.h"
#include "libbpf/src/bpf_helpers.h"
#include "libbpf/src/bpf_tracing.h"

#include "include/debug.h"
#include "include/ebpf_map.h"
#include "include/net_filter.h"
#include "include/net_helper.h"
#include "include/net_types.h"
#include "include/process.h"

#define PERF_MAX_STACK_DEPTH 16

// 过滤相关配置文件
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
  // u16 ip_version;
};

// 发送给用户态的event
struct perf_event_t {
  struct l2_info_t l2_info;
  struct l3_info_t l3_info;
  struct l4_info_t l4_info;
  struct pkt_info_t pkt_info;
  // struct process_info_t process_info;

  s64 stack_size;
  u64 stack[PERF_MAX_STACK_DEPTH];
  u8 drop_reason;
};

// 套接字关联信息
struct sock_owner_t {
  u64 pid_tgid;             // 进程pid_tgid
  char comm[TASK_COMM_LEN]; // 进程名称
  __be16 eth_proto;
  u16 ip_proto;
};

static const u8 config_key = 0;
static const struct perf_event_t zero_event = {};
static const u32 event_heap_key = 0;

// 存放config的hashmap
BPF_HASH_MAP_DEF(config_map, u8, struct config);

BPF_PERF_EVENT_ARRAY_DEF(perf_events);

// event_storage 是 percpuarray类型的map,
// 由于ebpf有栈大小限制,perf_event_t大结构体存放在map上面
BPF_PERCPU_ARRAY_DEF(event_heap, struct perf_event_t, 1);

// 存放套接字相关连的信息，出去报文的skb 通过获取关联socket来获取它正确的
// command信息
BPF_HASH_MAP_DEF(sock_owner_map, u64 /*struct sock的地址*/,
                 struct sock_owner_t);

static __always_inline struct sock_owner_t *
lookup_egress_sock_ref(struct sk_buff *skb) {
  struct sock *sk = (struct sock *)skb_sock(skb);
  if (!sk)
    return NULL;
  u64 key = (u64)sk;
  struct sock_owner_t *owner = bpf_map_lookup_elem(&sock_owner_map, &key);
  return owner;
}

// 基于l2 过滤
static __always_inline bool l2_filter(struct l2_info_t *l2,
                                      struct config *cfg) {
  // 发送路径的socket包的eth_proto是0，因此这里如果捕获到是0，大概率可能是发送路径的socket，可以直接放行，进行后续的过滤
  if (l2->eth_proto == 0)
    return true;
  return cfg->eth_proto == l2->eth_proto;
}

// 基于三层协议相关数据过滤， 主要检测ip地址是否匹配,
// 以及4层协议(tcp/udp)是否匹配 如果 config.host 为0，则
static __always_inline bool l3_filter(struct l3_info_t *l3, struct config *cfg,
                                      u8 ipvs) {
  /* if (cfg->ip_proto != 0 && cfg->ip_proto != l3->ip_proto)
    return false; */
  if (!ipaddr_is_zero(cfg->any_addr))
    return filter_match_any_ip(cfg->any_addr, l3->saddr, l3->daddr, ipvs);

  if (!filter_match_exact_ip(cfg->dst_addr, l3->daddr, ipvs))
    return false;
  if (!filter_match_exact_ip(cfg->src_addr, l3->saddr, ipvs))
    return false;

  if (ipaddr_is_zero(cfg->dst_addr) && ipaddr_is_zero(cfg->src_addr))
    return true;

  return true;
}

// 基于4层协议相关数据过滤,  主要检测端口是否匹配
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

static __always_inline bool
do_trace_ingress_skbdrop(void *ctx, struct config *cfg, struct sk_buff *skb,
                         struct perf_event_t *event) {

  set_l2_info(skb, &event->l2_info);

  unsigned char *network_header = skb_network_header(skb);
  u8 ipvs = ip_version(network_header);
  if (ipvs == 4)
    set_ipv4_info(skb, &event->l3_info);
  else if (ipvs == 6)
    set_ipv6_info(skb, &event->l3_info);
  else
    return BPF_OK;

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

SEC("kprobe/kfree_skb")
int kp_kfree_skb(struct pt_regs *ctx) {
  struct sk_buff *skb = (struct sk_buff *)PT_REGS_PARM1(ctx);
  struct perf_event_t *event = NULL;
  struct config *cfg =
      (struct config *)bpf_map_lookup_elem(&config_map, &config_key);

  if (!skb || !cfg)
    return BPF_OK;

  event = bpf_map_lookup_elem(&event_heap, &event_heap_key);
  if (!event)
    return BPF_OK;

  if (!do_trace_ingress_skbdrop(ctx, cfg, skb, event))
    return BPF_OK;

  bpf_perf_event_output(ctx, &perf_events, BPF_F_CURRENT_CPU, event,
                        sizeof(*event));
  // 防止脏数据,由于结构体过大使用`__builtin_memset__`初始化会被ebpf
  // 验证器给拒绝掉,所以这里使用一个归零数据来做初始化
  bpf_map_update_elem(&event_heap, &event_heap_key, &zero_event, BPF_EXIST);

  return BPF_OK;
}

char _license[] SEC("license") = "GPL";
