#ifndef OPENTRACE_BPF_EBPF_MAP_H
#define OPENTRACE_BPF_EBPF_MAP_H

#include "vmlinux.h"
#include "libbpf/src/bpf_helpers.h"

/*
 * 定义一个通用 HASH map:
 * - 默认最大容量为 65536
 */
#define BPF_HASH_MAP_DEF(name, key_type, value_type)                           \
  struct {                                                                     \
    __uint(type, BPF_MAP_TYPE_HASH);                                           \
    __uint(key_size, sizeof(key_type));                                        \
    __uint(value_size, sizeof(value_type));                                    \
    __uint(max_entries, 65536);                                                \
  } name SEC(".maps");

/*
 * 定义 perf event array，用于把 eBPF 事件上报到用户态。
 */
#define BPF_PERF_EVENT_ARRAY_DEF(name)                                         \
  struct {                                                                     \
    __uint(type, BPF_MAP_TYPE_PERF_EVENT_ARRAY);                               \
    __uint(key_size, sizeof(u32));                                             \
    __uint(value_size, sizeof(u32));                                           \
    __uint(max_entries, 0);                                                    \
  } name SEC(".maps");

/*
 *  定义
 */
#define BPF_PERCPU_ARRAY_DEF(name, value_type, entries)                        \
  struct {                                                                     \
    __uint(type, BPF_MAP_TYPE_PERCPU_ARRAY);                                   \
    __uint(key_size, sizeof(u32));                                             \
    __uint(value_size, sizeof(value_type));                                    \
    __uint(max_entries, entries);                                              \
  } name SEC(".maps");

/*
 * 定义一个BPF_MAP_TYPE_STACK_TRACE
 * - 默认最大容量为 65536
 *   内核版本v4.6以上
 */
#define BPF_STACK_TRACE_DEF(name)                                              \
  struct {                                                                     \
    __uint(type, BPF_MAP_TYPE_STACK_TRACE);                                    \
    __uint(key_size, sizeof(u32));                                             \
    __uint(value_size, sizeof(u64));                                           \
    __uint(max_entries, 65536);                                                \
  } name SEC(".maps");

/*
 * bpfmal类型: BPF_MAP_TYPE_RINGBUF
 * 文档:https://docs.ebpf.io/linux/map-type/BPF_MAP_TYPE_RINGBUF/
 * 内核支持版本:v5.8
 */
#define BPF_RINGBUF_DEF(name)                                                  \
  struct {                                                                     \
    __uint(type, BPF_MAP_TYPE_RINGBUF);                                        \
    __uint(max_entries, 256 * 1024);                                           \
  } name SEC(".maps");

/*
 * 读取 HASH map 中指定 key 的值并删除该 key
 */
static __always_inline void *bpf_hash_map_pop(void *map, const void *key) {
  void *value = bpf_map_lookup_elem(map, key);
  bpf_map_delete_elem(map, key);
  return value;
}

#endif
