#include "vmlinux.h"

#include "libbpf/src/bpf_endian.h"
#include "libbpf/src/bpf_tracing.h"
#include "libbpf/src/bpf_helpers.h"

#include "include/bpf_compat.h"
#include "include/net_types.h"
#include "include/ebpf_map.h"
#include "include/sock_helper.h"
#include "include/debug.h"

#define MAX_BUFFER_SIZE 1024
#define MAX_IOVEC_ITER 10
// ---------------------------------------------------------------------------
// 对外数据结构 和 Maps  和Rust侧代码对齐
// ---------------------------------------------------------------------------
struct config_t {
  u32 tgid; // pid
};
struct event_t {
  char buffer[MAX_BUFFER_SIZE];
  union addr remote_addr;
  union addr local_addr;
  u64 timestamp; // event 创建时间，用于统计请求进来到处理借宿所消耗了多久
  u32 size;
  u32 pid;
  u32 fd;
  u32 conn_kind;
  u32 flow_direct; // 请求方向, 出去。还是进来的包
  u16 remote_port;
  u16 local_port;
};

enum conn_kind {
  UNKNOWN_CONN = 0,
  ACTIVE_CONN = 1,
  POSITIVE_CONN = 2,
};
enum flow_direction { UNKNOWN_DIRECT = 0, INGRESS = 1, EGRESS = 2 };
// ---------------------------------------------------------------------------
// 内部使用数据结构
// ---------------------------------------------------------------------------

// connect 和 accept cache map公用
struct syscall_conn_args_t {
  int fd;
  struct sock *sk;
};

struct syscall_accept_args_t {
  int fd;
  struct socket *skt;
  struct sockaddr *addr;
};

struct syscall_rw_args_t {
  void *ubuf;
  struct iovec *iovec;
  size_t iovlen;
  u32 fd;
  int is_socket;
  enum flow_direction direction;
};

struct conn_info_key {
  u32 tgid;
  u32 fd;
};

struct conn_info_value {
  union addr local_addr;
  union addr remote_addr;
  u16 local_port;
  __be16 remote_port;
  short unsigned int family;
  int conn_kind; // 0 -> acive, 1 -> positive
};
// ---------------------------------------------------------------------------
// 对外使用的maps, rust侧代码会使用到这些maps
// perf_events： 将event事件发送给用户态代码
// config_map: 用户态传递参数到内核态
// ---------------------------------------------------------------------------

BPF_PERF_EVENT_ARRAY_DEF(perf_events);

BPF_HASH_MAP_DEF(config_map, u8, struct config_t);

// ---------------------------------------------------------------------------
// 辅助类的maps
// ---------------------------------------------------------------------------

BPF_PERCPU_ARRAY_DEF(event_heap, struct event_t, 1);

// sys_enter_* 和 sys_exit_* 之间cache
BPF_HASH_MAP_DEF(syscall_conn_args, u32, struct syscall_conn_args_t);
// sys_accept 和 sys_exit_accept之间的cache
BPF_HASH_MAP_DEF(syscall_accept_args, u32, struct syscall_accept_args_t);

// sys_read/sys_recv..等读路径上系统调用的一些参数存储到`syscall_read_args`这个hashmap里面
//  主要 里面存储buffer 指针和 buffer大小
BPF_HASH_MAP_DEF(syscall_read_args, u32, struct syscall_rw_args_t);

BPF_HASH_MAP_DEF(syscall_close_args, u32 /*tgid*/, u32 /*fd*/);
// sys_write/sys_send..等读路径上系统调用的一些参数存储到`syscall_write_args`这个hashmap里面
//  主要 里面存储buffer 指针和 buffer大小
BPF_HASH_MAP_DEF(syscall_write_args, u32, struct syscall_rw_args_t);

BPF_HASH_MAP_DEF(conn_info_map, struct conn_info_key, struct conn_info_value);

// ---------------------------------------------------------------------------
//           help functions
// ---------------------------------------------------------------------------

enum rw_buf_kind {
  RW_BUF_UBUF,
  RW_BUF_IOVEC,
  RW_BUF_MSG,
  RW_BUF_MMSG,
};

static __always_inline void populate_conn_info(const struct sock *sk,
                                               enum conn_kind conn_kind,
                                               struct conn_info_value *value) {
  u16 family = sock_family(sk);
  sock_remote_addr(sk, family, &value->remote_addr);
  sock_local_addr(sk, family, &value->local_addr);
  sock_local_port(sk, &value->local_port);
  sock_remote_port(sk, &value->remote_port);
  value->family = family;
  value->conn_kind = (int)conn_kind;
}

static __always_inline bool filter_with_pid(u32 pid) {
  u8 key = 0;
  struct config_t *cfg = bpf_map_lookup_elem(&config_map, &key);
  if (!cfg)
    return false;
  return cfg->tgid == pid;
}

// ---------------------------------------------------------------------------

// 将sys_read/sys_recv..等读路径上系统调用的一些参数存储到`syscall_read_args`这个hashmap里面
static __always_inline void stash_active_rw_args(int fd, void *ubuf,
                                                 struct iovec *iovec,
                                                 size_t iovlen,
                                                 void *args_map) {
  u32 tgid = bpf_get_current_pid_tgid() >> 32;
  struct conn_info_key key = {.tgid = tgid, .fd = fd};
  struct conn_info_value *value = bpf_map_lookup_elem(&conn_info_map, &key);
  if (!value) {
    return;
  }
  struct syscall_rw_args_t args;
  __builtin_memset(&args, 0, sizeof(args));
  args.fd = fd;
  args.ubuf = ubuf;
  args.is_socket = 1;
  args.iovec = iovec;
  args.iovlen = iovlen;
  bpf_map_update_elem(args_map, &tgid, &args, BPF_ANY);
}

// ---------------------------------------------------------------------------
static __always_inline void fill_event_from_ubuf(struct event_t *event,
                                                 char *buf, size_t size) {
  // 截到 event->buffer 容量内，给 verifier 一个明确上界
  size_t min_size = size;
  if (min_size > sizeof(event->buffer))
    min_size = sizeof(event->buffer);
  event->size = min_size;

  if (min_size > 0)
    bpf_probe_read_user(event->buffer, min_size, buf);
  event->size = min_size;
}

// 这个地方只迭代一次只抓第一个iovec 的数据，剩下的全部丢弃
// for chunk不太好处理，ebpfverify 验证比较严格, 后续想办法再处理
static __always_inline void fill_event_from_iovec(struct event_t *event,
                                                  struct iovec *vec,
                                                  size_t iovlen,
                                                  size_t total_bytes) {
  event->size = 0;
  if (iovlen == 0 || vec == NULL || total_bytes == 0)
    return;

  struct iovec iov = {};
  bpf_probe_read_user(&iov, sizeof(iov), vec);
  u32 to_read = iov.iov_len < total_bytes ? iov.iov_len : total_bytes;
  if (to_read > MAX_BUFFER_SIZE)
    to_read = MAX_BUFFER_SIZE - 1;
  to_read &= (MAX_BUFFER_SIZE - 1);
  if (to_read > 0)
    bpf_probe_read_user(event->buffer, to_read, iov.iov_base);
  event->size = to_read;
}

// 处理sys_enter_read/sys_enter_recv...等读路径上tracepoint
// enter时候的一些数据，主要是吧读取的buffer指针存放到map里面
// 如果buffer类型是msg/mmsg的话则转换成 iovec
static __always_inline void
process_syscall_enter(struct trace_event_raw_sys_enter *ctx,
                      enum rw_buf_kind kind, void *args_map) {
  int fd = ctx->args[0];
  if (kind == RW_BUF_UBUF) {
    char *ubuf = (char *)ctx->args[1];
    stash_active_rw_args(fd, ubuf, NULL, 0, args_map);
    return;
  } else if (kind == RW_BUF_IOVEC) {
    struct iovec *vec = (struct iovec *)ctx->args[1];
    unsigned long vlen = ctx->args[2];
    stash_active_rw_args(fd, NULL, vec, vlen, args_map);
    return;
  }

  struct iovec *iovec = NULL;
  size_t iovlen = 0;

  if (kind == RW_BUF_MSG) {
    struct user_msghdr *msg = (struct user_msghdr *)ctx->args[1];
    if (!msg)
      return;
    bpf_probe_read_user(&iovec, sizeof(iovec),
                        (void *)msg + offsetof(struct user_msghdr, msg_iov));
    bpf_probe_read_user(&iovlen, sizeof(iovlen),
                        (void *)msg + offsetof(struct user_msghdr, msg_iovlen));
  } else {
    struct mmsghdr *mmsg = (struct mmsghdr *)ctx->args[1];
    unsigned int vlen = ctx->args[2];
    if (!mmsg || vlen <= 0)
      return;
    bpf_probe_read_user(&iovec, sizeof(iovec),
                        (void *)mmsg + offsetof(struct mmsghdr, msg_hdr) +
                            offsetof(struct user_msghdr, msg_iov));
    bpf_probe_read_user(&iovlen, sizeof(iovlen),
                        (void *)mmsg + offsetof(struct mmsghdr, msg_hdr) +
                            offsetof(struct user_msghdr, msg_iovlen));
  }
  stash_active_rw_args(fd, NULL, iovec, iovlen, args_map);
}

// 读路径上tracepoint exit时候相关逻辑处理
// tracepoint exit 返回正常的时候，则将read数据submit到用户态
static __always_inline void
process_syscall_exit(struct trace_event_raw_sys_exit *ctx,
                     enum rw_buf_kind kind, void *args_map) {
  u32 tgid = bpf_get_current_pid_tgid() >> 32;
  u32 heap_idx = 0;
  struct syscall_rw_args_t *args = NULL;
  struct event_t *event = NULL;
  size_t total_read_bytes = ctx->ret;
  if (total_read_bytes <= 0)
    return;
  args = bpf_map_lookup_elem(args_map, &tgid);
  if (!args || args->is_socket != 1)
    return;
  struct conn_info_key key = {.tgid = tgid, .fd = args->fd};
  struct conn_info_value *value = bpf_map_lookup_elem(&conn_info_map, &key);
  if (value == NULL)
    return;

  event = bpf_map_lookup_elem(&event_heap, &heap_idx);
  if (event == NULL)
    return;

  event->timestamp = bpf_ktime_get_ns();
  event->pid = tgid;
  event->fd = args->fd;
  event->conn_kind = value->conn_kind;
  event->remote_addr = value->remote_addr;
  event->local_addr = value->local_addr;
  event->flow_direct = args->direction;
  event->remote_port = bpf_ntohs(value->remote_port);
  event->local_port = value->local_port;

  if (kind == RW_BUF_UBUF)
    fill_event_from_ubuf(event, args->ubuf, total_read_bytes);
  else if (kind == RW_BUF_IOVEC)
    fill_event_from_iovec(event, args->iovec, args->iovlen, total_read_bytes);

  bpf_perf_event_output(ctx, &perf_events, BPF_F_CURRENT_CPU, event,
                        sizeof(*event));
  bpf_map_delete_elem(args_map, &tgid);
}
// ---------------------------------------------------------------------------
//            tcp connect  获取tgid&fd 和 socket五元组的映射关系
// ---------------------------------------------------------------------------
SEC("tracepoint/syscalls/sys_enter_connect")
int tp_sys_enter_connect(struct trace_event_raw_sys_enter *ctx) {
  u32 tgid = bpf_get_current_pid_tgid() >> 32;
  if (filter_with_pid(tgid) == false)
    return BPF_OK;
  int fd = ctx->args[0];
  struct syscall_conn_args_t args;
  __builtin_memset(&args, 0, sizeof(args));
  args.fd = fd;
  args.sk = NULL;
  bpf_map_update_elem(&syscall_conn_args, &tgid, &args, BPF_ANY);
  return BPF_OK;
}

// int tcp_v4_connect(struct sock *sk, struct sockaddr *uaddr, int addr_len);
SEC("kprobe/tcp_connect")
int kp_tcp_connect(struct pt_regs *ctx) {
  u32 tgid = bpf_get_current_pid_tgid() >> 32;
  if (filter_with_pid(tgid) == false)
    return BPF_OK;
  struct sock *sk = (struct sock *)PT_REGS_PARM1(ctx);
  struct syscall_conn_args_t *args =
      bpf_map_lookup_elem(&syscall_conn_args, &tgid);
  if (!args)
    return BPF_OK;
  args->sk = sk;
  return BPF_OK;
}

// tcp_v4_connect| tcp_v6_connect
SEC("kretprobe/tcp_connect")
int kret_tcp_connect(struct pt_regs *ctx) {
  u32 tgid = bpf_get_current_pid_tgid() >> 32;
  int ret = (int)PT_REGS_RET(ctx);
  if (ret < 0) {
    bpf_map_delete_elem(&syscall_conn_args, &tgid);
    return BPF_OK;
  }
  return BPF_OK;
}

SEC("tracepoint/syscalls/sys_exit_connect")
int tp_sys_exit_connect(struct trace_event_raw_sys_exit *ctx) {
  u32 tgid = bpf_get_current_pid_tgid() >> 32;
  long ret = ctx->ret;

  if (ret < 0) {
    bpf_map_delete_elem(&syscall_conn_args, &tgid);
    return BPF_OK;
  }
  struct syscall_conn_args_t *args =
      bpf_map_lookup_elem(&syscall_conn_args, &tgid);
  if (!args)
    return BPF_OK;

  // todo  解析struct sock* 更新 connection maps
  struct conn_info_value value = {};
  struct conn_info_key key = {.tgid = tgid, .fd = args->fd};
  populate_conn_info(args->sk, ACTIVE_CONN, &value);
  bpf_map_update_elem(&conn_info_map, &key, &value, BPF_ANY);

  // 清理
  bpf_map_delete_elem(&syscall_conn_args, &tgid);
  return BPF_OK;
}

// ---------------------------------------------------------------------------
//       tcp accept 获取tgid&fd 和 socket五元组的映射关系
// ---------------------------------------------------------------------------
// 同时hook accept/accept4
SEC("tracepoint/syscalls/sys_enter_accept")
int tp_sys_enter_accept(struct trace_event_raw_sys_enter *ctx) {
  u32 tgid = bpf_get_current_pid_tgid() >> 32;
  if (filter_with_pid(tgid) == false)
    return BPF_OK;
  struct sockaddr *uservadd = (struct sockaddr *)ctx->args[1];
  // 这个地址存储下来主要是用备用，万一sock_alloc由于某些原因没有被触发的时候，这个时候可以拿着这个地址来解析远程的地址信息
  struct syscall_accept_args_t args;
  __builtin_memset(&args, 0, sizeof(args));
  args.fd = 0;
  args.skt = NULL;
  args.addr = uservadd;
  bpf_map_update_elem(&syscall_accept_args, &tgid, &args, BPF_ANY);
  return BPF_OK;
}

SEC("kretprobe/sock_alloc")
int kret_sock_alloc(struct pt_regs *ctx) {
  u32 tgid = bpf_get_current_pid_tgid() >> 32;
  struct syscall_accept_args_t *args =
      (struct syscall_accept_args_t *)bpf_map_lookup_elem(&syscall_accept_args,
                                                          &tgid);
  if (!args)
    return BPF_OK;
  struct socket *skt = (struct socket *)PT_REGS_RC(ctx);
  args->skt = skt;
  return BPF_OK;
}

SEC("tracepoint/syscalls/sys_exit_accept")
int tp_sys_exit_accept(struct trace_event_raw_sys_exit *ctx) {
  u64 tgid = bpf_get_current_pid_tgid() >> 32;
  int fd = ctx->ret;
  if (fd < 0) {
    bpf_map_delete_elem(&syscall_accept_args, &tgid);
    return BPF_OK;
  }

  struct syscall_accept_args_t *args =
      (struct syscall_accept_args_t *)bpf_map_lookup_elem(&syscall_accept_args,
                                                          &tgid);
  if (!args)
    return BPF_OK;
  args->fd = fd;

  struct conn_info_value value = {};
  struct conn_info_key key = {.fd = fd, .tgid = tgid};

  struct sock *sk = NULL;
  bpf_probe_read_kernel(&sk, sizeof(sk),
                        (void *)args->skt + offsetof(struct socket, sk));
  populate_conn_info(sk, POSITIVE_CONN, &value);
  bpf_map_update_elem(&conn_info_map, &key, &value, BPF_ANY);

  bpf_map_delete_elem(&syscall_accept_args, &tgid);

  return BPF_OK;
}

// ---------------------------------------------------------------------------
//      TCP receive mesage 获取socket读取的业务数据，通过perf_event 发送给用户态
// ---------------------------------------------------------------------------
//
//
// ---------------------------------------------------------------------------
//      read 一些列的tracepoint 为了 获取 socket 的五元组
// ---------------------------------------------------------------------------

SEC("tracepoint/syscalls/sys_enter_read")
int tp_sys_enter_read(struct trace_event_raw_sys_enter *ctx) {
  process_syscall_enter(ctx, RW_BUF_UBUF, &syscall_read_args);
  return BPF_OK;
}

SEC("tracepoint/syscalls/sys_exit_read")
int tp_sys_exit_read(struct trace_event_raw_sys_exit *ctx) {
  process_syscall_exit(ctx, RW_BUF_UBUF, &syscall_read_args);
  return BPF_OK;
}
// ---------------------------------------------------------------------------

SEC("tracepoint/syscalls/sys_enter_readv")
int tp_sys_enter_readv(struct trace_event_raw_sys_enter *ctx) {
  process_syscall_enter(ctx, RW_BUF_IOVEC, &syscall_read_args);
  return BPF_OK;
}

SEC("tracepoint/syscalls/sys_exit_readv")
int tp_sys_exit_readv(struct trace_event_raw_sys_exit *ctx) {
  process_syscall_exit(ctx, RW_BUF_IOVEC, &syscall_read_args);
  return BPF_OK;
}

// ---------------------------------------------------------------------------
SEC("tracepoint/syscalls/sys_enter_recv")
int tp_sys_enter_recv(struct trace_event_raw_sys_enter *ctx) {
  process_syscall_enter(ctx, RW_BUF_UBUF, &syscall_read_args);
  return BPF_OK;
}

SEC("tracepoint/syscalls/sys_exit_recv")
int tp_sys_exit_recv(struct trace_event_raw_sys_exit *ctx) {
  process_syscall_exit(ctx, RW_BUF_UBUF, &syscall_read_args);
  return BPF_OK;
}
// ---------------------------------------------------------------------------
SEC("tracepoint/syscalls/sys_enter_recvfrom")
int tp_sys_enter_recvfrom(struct trace_event_raw_sys_enter *ctx) {
  process_syscall_enter(ctx, RW_BUF_UBUF, &syscall_read_args);
  return BPF_OK;
}

SEC("tracepoint/syscalls/sys_exit_recvfrom")
int tp_sys_exit_recvfrom(struct trace_event_raw_sys_exit *ctx) {
  process_syscall_exit(ctx, RW_BUF_UBUF, &syscall_read_args);
  return BPF_OK;
}
// ---------------------------------------------------------------------------

SEC("tracepoint/syscalls/sys_enter_recvmsg")
int tp_sys_enter_recvmsg(struct trace_event_raw_sys_enter *ctx) {
  process_syscall_enter(ctx, RW_BUF_MSG, &syscall_read_args);
  return BPF_OK;
}

SEC("tracepoint/syscalls/sys_exit_recvmsg")
int tp_sys_exit_recvmsg(struct trace_event_raw_sys_exit *ctx) {
  process_syscall_exit(ctx, RW_BUF_IOVEC, &syscall_read_args);
  return BPF_OK;
}

// ---------------------------------------------------------------------------

SEC("tracepoint/syscalls/sys_enter_recvmmsg")
int tp_sys_enter_recvmmsg(struct trace_event_raw_sys_enter *ctx) {
  process_syscall_enter(ctx, RW_BUF_MMSG, &syscall_read_args);
  return BPF_OK;
}

SEC("tracepoint/syscalls/sys_exit_recvmmsg")
int tp_sys_exit_recvmmsg(struct trace_event_raw_sys_exit *ctx) {
  process_syscall_exit(ctx, RW_BUF_IOVEC, &syscall_read_args);
  return BPF_OK;
}

// ---------------------------------------------------------------------------
// int security_socket_recvmsg(struct socket *sock, struct msghdr *msg,
SEC("kprobe/security_socket_recvmsg")
int kp_security_socket_recvmsg(struct pt_regs *ctx) {
  u32 tgid = bpf_get_current_pid_tgid() >> 32;
  struct syscall_rw_args_t *args =
      bpf_map_lookup_elem(&syscall_read_args, &tgid);
  if (!args)
    return BPF_OK;
  args->is_socket = 1;
  args->direction = INGRESS;
  return BPF_OK;
}

// ---------------------------------------------------------------------------
//       tcp send 获取socket写入的业务数据，通过perf_event 发送给用户态
// ---------------------------------------------------------------------------
SEC("kprobe/security_socket_sendmsg")
int kp_security_socket_sendmsg(struct pt_regs *ctx) {
  u32 tgid = bpf_get_current_pid_tgid() >> 32;
  struct syscall_rw_args_t *args =
      bpf_map_lookup_elem(&syscall_write_args, &tgid);
  if (!args)
    return BPF_OK;
  args->is_socket = 1;
  args->direction = EGRESS;
  return BPF_OK;
}

// ssize_t write(int fd, const void *buf, size_t count);
SEC("tracepoint/syscalls/sys_enter_write")
int tp_sys_enter_write(struct trace_event_raw_sys_enter *ctx) {
  process_syscall_enter(ctx, RW_BUF_UBUF, &syscall_write_args);
  return BPF_OK;
}

SEC("tracepoint/syscalls/sys_exit_write")
int tp_sys_exit_write(struct trace_event_raw_sys_exit *ctx) {
  process_syscall_exit(ctx, RW_BUF_UBUF, &syscall_write_args);
  return BPF_OK;
}

// ---------------------------------------------------------------------------
// ssize_t writev(int fd, const struct iovec *iov, int iovcnt);
SEC("tracepoint/syscalls/sys_enter_writev")
int tp_sys_enter_writev(struct trace_event_raw_sys_enter *ctx) {
  process_syscall_enter(ctx, RW_BUF_IOVEC, &syscall_write_args);
  return BPF_OK;
}

SEC("tracepoint/syscalls/sys_exit_writev")
int tp_sys_exit_writev(struct trace_event_raw_sys_exit *ctx) {
  process_syscall_exit(ctx, RW_BUF_IOVEC, &syscall_write_args);
  return BPF_OK;
}
// ---------------------------------------------------------------------------
// ssize_t send(int sockfd, const void *buf, size_t len, int flags);
SEC("tracepoint/syscalls/sys_enter_send")
int tp_sys_enter_send(struct trace_event_raw_sys_enter *ctx) {
  process_syscall_enter(ctx, RW_BUF_UBUF, &syscall_write_args);
  return BPF_OK;
}

SEC("tracepoint/syscalls/sys_exit_send")
int tp_sys_exit_send(struct trace_event_raw_sys_exit *ctx) {
  process_syscall_exit(ctx, RW_BUF_UBUF, &syscall_write_args);
  return BPF_OK;
}
// ---------------------------------------------------------------------------
// ssize_t sendto(int sockfd, const void *buf, size_t len, int flags, const
// struct sockaddr *dest_addr, socklen_t addrlen);
SEC("tracepoint/syscalls/sys_enter_sendto")
int tp_sys_enter_sendto(struct trace_event_raw_sys_enter *ctx) {
  process_syscall_enter(ctx, RW_BUF_IOVEC, &syscall_write_args);
  return BPF_OK;
}

SEC("tracepoint/syscalls/sys_exit_sendto")
int tp_sys_exit_sendto(struct trace_event_raw_sys_exit *ctx) {
  process_syscall_exit(ctx, RW_BUF_IOVEC, &syscall_write_args);
  return BPF_OK;
}
// ---------------------------------------------------------------------------
// ssize_t sendmsg(int sockfd, const struct msghdr *msg, int flags);
SEC("tracepoint/syscalls/sys_enter_sendmsg")
int tp_sys_enter_sendmsg(struct trace_event_raw_sys_enter *ctx) {
  process_syscall_enter(ctx, RW_BUF_MSG, &syscall_write_args);
  return BPF_OK;
}

SEC("tracepoint/syscalls/sys_exit_sendmsg")
int tp_sys_exit_sendmsg(struct trace_event_raw_sys_exit *ctx) {
  process_syscall_exit(ctx, RW_BUF_MSG, &syscall_write_args);
  return BPF_OK;
}
// ---------------------------------------------------------------------------
// int sendmmsg(int sockfd, struct mmsghdr *msgvec, unsigned int vlen, int
// flags);
SEC("tracepoint/syscalls/sys_enter_sendmmsg")
int tp_sys_enter_sendmmsg(struct trace_event_raw_sys_enter *ctx) {
  process_syscall_enter(ctx, RW_BUF_MMSG, &syscall_write_args);
  return BPF_OK;
}

SEC("tracepoint/syscalls/sys_exit_sendmmsg")
int tp_sys_exit_sendmmsg(struct trace_event_raw_sys_exit *ctx) {
  process_syscall_exit(ctx, RW_BUF_MMSG, &syscall_write_args);
  return BPF_OK;
}

// ---------------------------------------------------------------------------
//       tcp close
// ---------------------------------------------------------------------------

SEC("tracepoint/syscalls/sys_enter_close")
int tp_sys_enter_close(struct trace_event_raw_sys_enter *ctx) {
  u64 tgid = bpf_get_current_pid_tgid() >> 32;
  u64 fd = ctx->args[0];
  bpf_map_update_elem(&syscall_close_args, &tgid, &fd, BPF_ANY);
  return BPF_OK;
}

// 不论enter_close是否成功，这个应该都应该
SEC("tracepoint/syscalls/sys_exit_close")
int tp_sys_exit_close(struct trace_event_raw_sys_exit *ctx) {
  u64 tgid = bpf_get_current_pid_tgid() >> 32;
  long ret = ctx->ret;
  if (ret < 0)
    return BPF_OK;
  u32 *fd = bpf_map_lookup_elem(&syscall_close_args, &tgid);
  if (!fd)
    return BPF_OK;
  struct conn_info_key key = {.fd = *fd, .tgid = tgid};
  bpf_map_delete_elem(&conn_info_map, &key);

  return BPF_OK;
}
char _license[] SEC("license") = "GPL";
