## 注意
bpf/目录下 `*.skel.rs`文件均有`libbpf-crate`生成，不用手动去修改
## Tracepoint函数命名方式
```c
int tp_<tracepint名称>(){}
// 例如syscalls:sys_enter_recvmsg
// 那么它的h函数是 tp_sys_enter_recvmsg
int tp_sys_enter_recvmsg(struct trace_event_raw_sys_enter){}
```

```rust
macro_rules! attach_syscall_tp {
    ($name:ident) => {
        paste::paste! {
            if self
                .probe_registry
                .tracepoint_is_available(concat!("syscalls:", stringify!($name)))
            {
                let link = self
                    .skel
                    .progs
                    .[<tp_ $name>]
                    .attach_tracepoint(
                        libbpf_rs::TracepointCategory::Syscalls,
                        stringify!($name),
                    )?;
                self._links.push(link);
            }
        }
    };
}
```
> rust代码里面严格按照这命名格式来加载的

