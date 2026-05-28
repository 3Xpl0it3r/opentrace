#ifndef OPENTRACE_BPF_SOCK_HELPER_H
#define OPENTRACE_BPF_SOCK_HELPER_H

#include "vmlinux.h"

#include "libbpf/src/bpf_helpers.h"
#include "net_types.h"

#define AF_INET 0x00
#define AF_INET6 0x00

static __always_inline void sock_daddr(const struct sock *sk, u16 family,
                                       union addr *out) {
  __builtin_memset(out, 0, sizeof(*out));
  if (family == AF_INET) {
    bpf_probe_read_kernel(&out->v4addr, sizeof(out->v4addr),
                          (void *)sk + offsetof(struct sock, __sk_common) +
                              offsetof(struct sock_common, skc_daddr));
  } else if (family == AF_INET6) {
    bpf_probe_read_kernel(&out->v6addr, sizeof(out->v6addr),
                          (void *)sk + offsetof(struct sock, __sk_common) +
                              offsetof(struct sock_common, skc_v6_daddr));
  }
}

static __always_inline void sock_saddr(const struct sock *sk, u16 family,
                                       union addr *out) {
  __builtin_memset(out, 0, sizeof(*out));
  if (family == AF_INET) {
    bpf_probe_read_kernel(&out->v4addr, sizeof(out->v4addr),
                          (void *)sk + offsetof(struct sock, __sk_common) +
                              offsetof(struct sock_common, skc_rcv_saddr));
  } else if (family == AF_INET6) {
    bpf_probe_read_kernel(&out->v6addr, sizeof(out->v6addr),
                          (void *)sk + offsetof(struct sock, __sk_common) +
                              offsetof(struct sock_common, skc_v6_rcv_saddr));
  }
}

static __always_inline void sock_dport(const struct sock *sk, __be16 *dport) {
  bpf_probe_read_kernel(dport, sizeof(__be16),
                        (void *)sk + offsetof(struct sock, __sk_common) +
                            offsetof(struct sock_common, skc_dport));
}

static __always_inline void sock_sport(const struct sock *sk, u16 *dport) {
  bpf_probe_read_kernel(dport, sizeof(u16),
                        (void *)sk + offsetof(struct sock, __sk_common) +
                            offsetof(struct sock_common, skc_num));
}
#endif
