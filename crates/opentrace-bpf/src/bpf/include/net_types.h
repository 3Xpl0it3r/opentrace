#ifndef OPENTRACE_BPF_NET_TYPES_H
#define OPENTRACE_BPF_NET_TYPES_H

#include "vmlinux.h"

#define IFNAMSIZ 16
#define ETH_P_ALL 0x0003
#define ETH_P_IP 0x0800
#define ETH_P_IPV6 0x86dd
#define ETH_P_ARP 0x0806

struct addr_v6 {
  u64 upper;
  u64 lower;
};

// IP 地址
union addr {
  u32 v4addr;            // V4
  struct addr_v6 v6addr; // v6
};

// 2层网络报文相关信息
struct l2_info_t {
  __be16 eth_proto;
};

// 三层网络报文相关信息，保存源/目的地址、总长度和上层协议。
struct l3_info_t {
  union addr saddr;
  union addr daddr;
  u16 tot_len;
  u16 ip_version;
  u8 ip_proto;
};

// 四层网络报文相关信息，保存端口和 TCP flags。
struct l4_info_t {
  // 大端存储
  u16 sport;
  // 大端存储
  u16 dport;
  u16 tcpflags;
  u8 pad[2];
};

// 报文元信息，保存网卡、长度、CPU、进程和命名空间等字段。
struct pkt_info_t {
  char ifname[IFNAMSIZ];
  u32 len;
  u32 cpu;
  u32 pid;
  u32 netns;
  u8 pkt_type; // skb->pkt_type
};

#endif
