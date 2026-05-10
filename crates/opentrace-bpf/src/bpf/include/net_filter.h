#ifndef OPENTRACE_BPF_NET_FILTER_H
#define OPENTRACE_BPF_NET_FILTER_H

#include "vmlinux.h"
#include "libbpf/src/bpf_helpers.h"
#include "net_types.h"

// true则匹配
static __always_inline bool ipaddr_is_equal(union addr source, union addr target, u8 ip_version) {
  return ip_version == 4
             ? source.v4addr == target.v4addr
             : source.v6addr.lower == target.v6addr.lower && source.v6addr.upper == target.v6addr.upper;
}

static __always_inline bool ipaddr_is_zero(union addr ipaddr) {
  return ipaddr.v6addr.lower == 0 && ipaddr.v6addr.upper == 0;
}

// true则匹配
static __always_inline bool filter_match_any_ip(union addr target, union addr source, union addr dest, u8 ip_version) {
  if (ipaddr_is_zero(target))
    return true;

  return ipaddr_is_equal(target, source, ip_version) || ipaddr_is_equal(target, dest, ip_version);
}

static __always_inline bool filter_match_exact_ip(union addr target, union addr addr, u8 ip_version) {
  if (ipaddr_is_zero(target))
    return true;

  return ipaddr_is_equal(target, addr, ip_version);
}

static __always_inline bool filter_match_pid(u32 target_pid, u32 pid) {
  return target_pid == 0 || target_pid == pid;
}

static __always_inline bool filter_match_netns(u32 target_netns, u32 netns) {
  return target_netns == 0 || target_netns == netns;
}

#endif
