#ifndef OPENTRACE_BPF_DEBUG_H
#define OPENTRACE_BPF_DEBUG_H


#include "vmlinux.h"
#include "libbpf/src/bpf_helpers.h"
// 这个是用来debug的,把 IPv4 地址格式化为字符串,便于通过printk 输出网络层信息。
// char ip[16]
// be32_to_ipv4_str(xxx, ip);
static __always_inline void be32_to_ipv4_str(__be32 ip_value, char *out) {
  __u64 ip_data[4] = {
      (__u64)(ip_value & 0xFF),
      (__u64)((ip_value >> 8) & 0xFF),
      (__u64)((ip_value >> 16) & 0xFF),
      (__u64)((ip_value >> 24) & 0xFF)};
  bpf_snprintf(out, 16, "%d.%d.%d.%d", ip_data, 4 * sizeof(__u64));
}

#endif
