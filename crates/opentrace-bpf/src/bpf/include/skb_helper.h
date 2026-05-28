#ifndef OPENTRACE_BPF_SKB_HELPER_H
#define OPENTRACE_BPF_SKB_HELPER_H

#include "vmlinux.h"
#include "libbpf/src/bpf_helpers.h"
#include "libbpf/src/bpf_core_read.h"
#include "net_types.h"

// 从struct sk_buff里面获取struct sock字段
static __always_inline unsigned char *skb_sock(struct sk_buff *skb) {
  struct sock *sck = NULL;

  if (bpf_probe_read_kernel(&sck, sizeof(sck),
                            (void *)skb + offsetof(struct sk_buff, sk)) != 0)
    return NULL;

  return (unsigned char *)sck;
}

// 从sock里面获取网络协议
// 注意: 自 4.x 起 struct sock::sk_protocol 是 bit-field, 不能用 offsetof,
// 必须用 CO-RE 的 bit-field 读取宏 (由 libbpf 在加载时做 relocation)。
static __always_inline u16 sck_protocol(struct sock *sck) {
  return (u16)BPF_CORE_READ_BITFIELD_PROBED(sck, sk_protocol);
}

// 从sock里面获取网络协议簇, 是IPV4 还是IPV6
static __always_inline u16 sck_faimly(struct sock *sck) {
  short unsigned int skc_family = 0;
  bpf_probe_read_kernel(&skc_family, sizeof(skc_family),
                        (void *)sck + offsetof(struct sock, __sk_common) +
                            offsetof(struct sock_common, skc_family));
  return skc_family;
}

static __always_inline unsigned char *skb_head(struct sk_buff *skb) {
  unsigned char *head = NULL;

  if (bpf_probe_read_kernel(&head, sizeof(head),
                            (void *)skb + offsetof(struct sk_buff, head)) != 0)
    return NULL;

  return head;
}

static __always_inline unsigned char *skb_header_pointer(struct sk_buff *skb,
                                                         u16 offset) {
  unsigned char *head = skb_head(skb);

  if (!head)
    return NULL;

  return head + offset;
}

static __always_inline u16 skb_protocol(struct sk_buff *skb) {
  u16 protocol = 0;

  bpf_probe_read_kernel(&protocol, sizeof(protocol),
                        (void *)skb + offsetof(struct sk_buff, protocol));
  return protocol;
}

// 返回 skb 的以太网层头指针，失败时返回 NULL。
static __always_inline unsigned char *skb_ether_header(struct sk_buff *skb) {
  u16 mac_header = 0;

  if (bpf_probe_read_kernel(&mac_header, sizeof(mac_header),
                            (void *)skb +
                                offsetof(struct sk_buff, mac_header)) != 0)
    return NULL;

  return skb_header_pointer(skb, mac_header);
}

// 返回 skb 的网络层头指针，失败时返回 NULL。
static __always_inline unsigned char *skb_network_header(struct sk_buff *skb) {
  u16 network_header = 0;

  if (bpf_probe_read_kernel(&network_header, sizeof(network_header),
                            (void *)skb +
                                offsetof(struct sk_buff, network_header)) != 0)
    return NULL;

  return skb_header_pointer(skb, network_header);
}

// 返回 skb 的传输层头指针，失败时返回 NULL。
static __always_inline unsigned char *
skb_transport_header(struct sk_buff *skb) {
  u16 transport_header = 0;

  if (bpf_probe_read_kernel(
          &transport_header, sizeof(transport_header),
          (void *)skb + offsetof(struct sk_buff, transport_header)) != 0)
    return NULL;

  return skb_header_pointer(skb, transport_header);
}

// 根据网络层头的版本字段判断 L3 协议，4代表是ipv4, 6代表是ipv6
static __always_inline u8 ip_version(void *network_header) {
  u8 ver_ihl = 0;

  if (!network_header ||
      bpf_probe_read_kernel(&ver_ihl, sizeof(ver_ihl), network_header) != 0)
    return 0;

  return (ver_ihl >> 4) & 0x0f;
}

// 填充l2_info,
// 优先获取skb->protocol字段，失败则从eth_hdr里面获取,理论上这两个字段的值是一样的
static __always_inline void populate_l2_info(struct sk_buff *skb,
                                          struct l2_info_t *l2_info) {
  struct ethhdr *eth_header = (struct ethhdr *)skb_ether_header(skb);

  if (bpf_probe_read_kernel(&l2_info->eth_proto, sizeof(l2_info->eth_proto),
                            (void *)skb + offsetof(struct sk_buff, protocol)) ==
      0)
    return;

  if (eth_header &&
      bpf_probe_read_kernel(&l2_info->eth_proto, sizeof(l2_info->eth_proto),
                            &eth_header->h_proto) == 0)
    return;
}

// 从 skb 的 IPv4 头填充l3_info_t
static __always_inline void populate_ipv4_info(struct sk_buff *skb,
                                            struct l3_info_t *l3_info) {
  struct iphdr *iphdr = (struct iphdr *)skb_network_header(skb);
  l3_info->ip_version = 4;

  bpf_probe_read_kernel(&l3_info->ip_proto, sizeof(l3_info->ip_proto),
                        (void *)iphdr + offsetof(struct iphdr, protocol));
  bpf_probe_read_kernel(&l3_info->saddr.v4addr, sizeof(l3_info->saddr.v4addr),
                        (void *)iphdr + offsetof(struct iphdr, saddr));
  bpf_probe_read_kernel(&l3_info->daddr.v4addr, sizeof(l3_info->daddr.v4addr),
                        (void *)iphdr + offsetof(struct iphdr, daddr));
}

// 从 skb 的 IPv6 头填充l3_info_t
static __always_inline void populate_ipv6_info(struct sk_buff *skb,
                                            struct l3_info_t *l3_info) {
  struct ipv6hdr *iphdr = (struct ipv6hdr *)skb_network_header(skb);
  l3_info->ip_version = 6;

  bpf_probe_read_kernel(&l3_info->ip_proto, sizeof(l3_info->ip_proto),
                        (void *)iphdr + offsetof(struct ipv6hdr, nexthdr));
  bpf_probe_read_kernel(&l3_info->saddr.v6addr, sizeof(l3_info->saddr.v6addr),
                        (void *)iphdr + offsetof(struct ipv6hdr, saddr));
  bpf_probe_read_kernel(&l3_info->daddr.v6addr, sizeof(l3_info->daddr.v6addr),
                        (void *)iphdr + offsetof(struct ipv6hdr, daddr));
}

// 从 skb 的 TCP 头填充l4_info_t
static __always_inline void populate_tcp_info(struct sk_buff *skb,
                                         struct l4_info_t *l4_info) {
  struct tcphdr *tcphdr = (struct tcphdr *)skb_transport_header(skb);

  bpf_probe_read_kernel(&l4_info->sport, sizeof(l4_info->sport),
                        (void *)tcphdr + offsetof(struct tcphdr, source));
  bpf_probe_read_kernel(&l4_info->dport, sizeof(l4_info->dport),
                        (void *)tcphdr + offsetof(struct tcphdr, dest));
  bpf_probe_read_kernel(&l4_info->tcpflags, sizeof(l4_info->tcpflags),
                        (unsigned char *)tcphdr + 13);
}

// 从 skb 的 UDP 头填充l4_info_t
static __always_inline void populate_udp_info(struct sk_buff *skb,
                                         struct l4_info_t *l4_info) {
  struct udphdr *udphdr = (struct udphdr *)skb_transport_header(skb);

  bpf_probe_read_kernel(&l4_info->sport, sizeof(l4_info->sport),
                        (void *)udphdr + offsetof(struct udphdr, source));
  bpf_probe_read_kernel(&l4_info->dport, sizeof(l4_info->dport),
                        (void *)udphdr + offsetof(struct udphdr, dest));
}

// 填充报文调试信息，包括 CPU、进程、报文长度、网卡名和网络命名空间。
static __always_inline void populate_pkt_info(struct sk_buff *skb,
                                         struct pkt_info_t *pkt_info) {
  pkt_info->cpu = bpf_get_smp_processor_id();
  pkt_info->pid = bpf_get_current_pid_tgid() >> 32;

  if (bpf_probe_read_kernel(&pkt_info->len, sizeof(pkt_info->len),
                            (void *)skb + offsetof(struct sk_buff, len)) != 0)
    return;

  struct net_device *device = NULL;
  if (bpf_probe_read_kernel(&device, sizeof(device),
                            (void *)skb + offsetof(struct sk_buff, dev)) != 0)
    return;

  bpf_probe_read_kernel_str(&pkt_info->ifname, sizeof(pkt_info->ifname),
                            (void *)device + offsetof(struct net_device, name));

  struct net *net = NULL;
  if (bpf_probe_read_kernel(&net, sizeof(net),
                            (void *)device +
                                offsetof(struct net_device, nd_net)) != 0)
    return;

  bpf_probe_read_kernel(&pkt_info->netns, sizeof(pkt_info->netns),
                        (void *)net + offsetof(struct net, ns) +
                            offsetof(struct ns_common, inum));
}

#endif
