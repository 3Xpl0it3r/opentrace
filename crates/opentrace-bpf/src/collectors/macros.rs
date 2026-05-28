// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.

// 生成Colletor结构体，同时自动实现create:collectors:Collector trait
macro_rules! define_collector {
    ($name:ident, $skel:ident) => {
        pub struct $name<'a> {
            probe_registry: &'a $crate::ProbeRegistry,
            skel: $skel<'a>,
            perf_buffer: ::libbpf_rs::PerfBuffer<'a>,
            _links: Vec<::libbpf_rs::Link>,
        }

        impl<'a> $crate::collectors::Collector for $name<'a> {
            fn poll(
                &mut self,
                interval: ::std::time::Duration,
            ) -> ::std::result::Result<(), $crate::EbpfError> {
                let _ = self.perf_buffer.poll(interval);
                Ok(())
            }

            fn attach_probe(&mut self) -> ::std::result::Result<(), $crate::EbpfError> {
                // 这个需要用户自己实现
                self.do_attach_probes()
            }
        }
    };
}

// attach_tracepoint!(self, "syscalls", tp_sys_enter_connect)
macro_rules! attach_tracepoint {
    ($self:ident, $cat:literal, $prog:ident) => {{
        let (category, prefix) = match $cat {
            "syscalls" => (::libbpf_rs::TracepointCategory::Syscalls, "syscalls"),
            "net" => (::libbpf_rs::TracepointCategory::Net, "net"),
            other => panic!("attach_tracepoint: unsupported category {:?}", other),
        };
        let tp_name: &str = stringify!($prog)
            .strip_prefix("tp_")
            .expect("attach_tracepoint: prog ident must start with `tp_`");
        let lookup = ::std::format!("{}:{}", prefix, tp_name);
        if $self.probe_registry.tracepoint_is_available(&lookup) {
            let link = $self
                .skel
                .progs
                .$prog
                .attach_tracepoint(category, tp_name)?;
            $self._links.push(link);
        }
    }};
}

// attach_multiple_tracepoints!(self, "syscalls", tp_sys_enter, ["sys_enter_connect", "sys_enter_accept"]);
macro_rules! attach_multiple_tracepoints {
    ($self:ident, $cat:literal, $prog:ident, $tps:expr) => {{
        let (category, prefix) = match $cat {
            "syscalls" => (::libbpf_rs::TracepointCategory::Syscalls, "syscalls"),
            "net" => (::libbpf_rs::TracepointCategory::Net, "net"),
            other => panic!(
                "attach_multiple_tracepoints: unsupported category {:?}",
                other
            ),
        };
        for tp in ($tps).iter() {
            let tp_name: &str = ::std::convert::AsRef::<str>::as_ref(tp);
            let lookup = ::std::format!("{}:{}", prefix, tp_name);
            if $self.probe_registry.tracepoint_is_available(&lookup) {
                let link = $self
                    .skel
                    .progs
                    .$prog
                    .attach_tracepoint(category.clone(), tp_name)?;
                $self._links.push(link);
            }
        }
    }};
}

macro_rules! attach_kprobe {
    ($self:ident, $prog:ident, $func:expr) => {{
        let func = $func;
        if $self.probe_registry.kprobe_is_available(func) {
            let link = $self.skel.progs.$prog.attach_kprobe(false, func)?;
            $self._links.push(link);
        }
    }};
}

macro_rules! attach_kretprobe {
    ($self:ident, $prog:ident, $func:expr) => {{
        let func = $func;
        if $self.probe_registry.kprobe_is_available(func) {
            let link = $self.skel.progs.$prog.attach_kprobe(true, func)?;
            $self._links.push(link);
        }
    }};
}

// attach_multiple_kprobes!(self, kp_handler, ["tcp_connect", "tcp_close"]);
macro_rules! attach_multiple_kprobes {
    ($self:ident, $prog:ident, $funcs:expr) => {{
        for func in ($funcs).iter() {
            let func: &str = ::std::convert::AsRef::<str>::as_ref(func);
            if $self.probe_registry.kprobe_is_available(func) {
                let link = $self.skel.progs.$prog.attach_kprobe(false, func)?;
                $self._links.push(link);
            }
        }
    }};
}

// attach_multiple_kretprobes!(self, kret_handler, ["tcp_connect", "tcp_close"]);
// 同上，但挂到返回点（kretprobe）。
macro_rules! attach_multiple_kretprobes {
    ($self:ident, $prog:ident, $funcs:expr) => {{
        for func in ($funcs).iter() {
            let func: &str = ::std::convert::AsRef::<str>::as_ref(func);
            if $self.probe_registry.kprobe_is_available(func) {
                let link = $self.skel.progs.$prog.attach_kprobe(true, func)?;
                $self._links.push(link);
            }
        }
    }};
}

// attach_perf_event!(self, perf_profile_samples, pfd);
// 单次 attach；调用方负责遍历 fd 集合。$pfd 实参只要实现
// std::os::fd::AsRawFd 即可（典型是 &OwnedFd）。
macro_rules! attach_perf_event {
    ($self:ident, $prog:ident, $pfd:expr) => {{
        let link = $self
            .skel
            .progs
            .$prog
            .attach_perf_event(::std::os::fd::AsRawFd::as_raw_fd($pfd))?;
        $self._links.push(link);
    }};
}

pub(crate) use {
    attach_kprobe, attach_kretprobe, attach_multiple_kprobes, attach_multiple_kretprobes,
    attach_multiple_tracepoints, attach_perf_event, attach_tracepoint, define_collector,
};
