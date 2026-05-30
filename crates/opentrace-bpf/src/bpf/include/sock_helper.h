#ifndef OPENTRACE_BPF_SOCK_HELPER_H
#define OPENTRACE_BPF_SOCK_HELPER_H

#include "vmlinux.h"

#include "libbpf/src/bpf_helpers.h"
#include "net_types.h"

#ifndef AF_INET
#define AF_INET 2
#endif
#ifndef AF_INET6
#define AF_INET6 10
#endif

static __always_inline u16 sock_family(const struct sock *sk) {
  u16 family = 0;
  if (!sk)
    return family;

  bpf_probe_read_kernel(&family, sizeof(family),
                        (void *)sk + offsetof(struct sock, __sk_common) +
                            offsetof(struct sock_common, skc_family));
  return family;
}

static __always_inline void sock_remote_addr(const struct sock *sk, u16 family,
                                             union addr *out) {
  __builtin_memset(out, 0, sizeof(*out));
  if (!sk)
    return;

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

static __always_inline void sock_local_addr(const struct sock *sk, u16 family,
                                            union addr *out) {
  __builtin_memset(out, 0, sizeof(*out));
  if (!sk)
    return;

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

static __always_inline void sock_remote_port(const struct sock *sk,
                                             __be16 *dport) {
  *dport = 0;
  if (!sk)
    return;

  bpf_probe_read_kernel(dport, sizeof(*dport),
                        (void *)sk + offsetof(struct sock, __sk_common) +
                            offsetof(struct sock_common, skc_dport));
}

static __always_inline void sock_local_port(const struct sock *sk, u16 *port) {
  *port = 0;
  if (!sk)
    return;

  bpf_probe_read_kernel(port, sizeof(*port),
                        (void *)sk + offsetof(struct sock, __sk_common) +
                            offsetof(struct sock_common, skc_num));
}

static __always_inline void sock_daddr(const struct sock *sk, u16 family,
                                       union addr *out) {
  sock_remote_addr(sk, family, out);
}

static __always_inline void sock_saddr(const struct sock *sk, u16 family,
                                       union addr *out) {
  sock_local_addr(sk, family, out);
}

static __always_inline void sock_dport(const struct sock *sk, __be16 *dport) {
  sock_remote_port(sk, dport);
}

static __always_inline void sock_sport(const struct sock *sk, u16 *port) {
  sock_local_port(sk, port);
}
#endif
