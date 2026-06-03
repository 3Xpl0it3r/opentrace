#[cfg(target_arch = "x86_64")]
use std::ffi::{c_long, c_uint};
use std::os::fd::{FromRawFd as _, OwnedFd, RawFd};

use libbpf_rs::libbpf_sys::perf_event_attr;
use libc::syscall;

use crate::EbpfError;

// 这部分逻辑借鉴了 https://github.com/Phantomical/perf-event/ 项目

#[cfg(target_arch = "x86")]
pub const SYS_PERF_EVENT_OPEN: c_int = 336;

#[cfg(target_arch = "arm")]
pub const SYS_PERF_EVENT_OPEN: c_int = 364;

#[cfg(target_arch = "aarch64")]
pub const SYS_PERF_EVENT_OPEN: c_int = 241;

// 对于 x32 ABI 的特殊情况
#[cfg(all(target_arch = "x86_64", target_pointer_width = "32"))]
pub const SYS_PERF_EVENT_OPEN: c_int = 298 | 0x40000000;

const PERF_TYPE_SOFTWARE: c_uint = 1;
const PERF_COUNT_SW_CPU_CLOCK: u64 = 0;
/* const DEFAULT_SAMPLE_PERIOD:u64 = 1_000_000; // 每1/ms采样一次 */
const DEFAULT_SAMPLE_PERIOD: u64 = 10000;

#[cfg(target_os = "linux")]
pub const SYS_PERF_EVENT_OPEN: c_long = 298;
#[cfg(target_os = "macos")]
pub const SYS_PERF_EVENT_OPEN: c_int = 298;

// 用来构建perf fd
pub struct PerfEventFdBuilder {
    attrs: perf_event_attr,
    /// -1 代表所有的cpu (默认值)
    /// >=0 代表指定cpu
    cpu: i32,
    /// -1 代表所有进程
    /// 0 代表调用perf程序本身, 默认 (不支持pid=-1, cpu=-1)
    /// >0 用户指定进程
    tid: i32,
    group_id: i32,
}

// perf event其实是根据tid 去attach的，因此这个地方需要传tid进来
impl PerfEventFdBuilder {
    pub fn attach_tid(&mut self, tid: i32) {
        self.tid = tid
    }
    pub fn attach_cpu(&mut self, cpu: i32) {
        self.cpu = cpu
    }
    /* pub fn attach_groupid(&mut self, group_id: i32) {
        self.group_id = group_id
    } */

    pub fn build(&self) -> Result<OwnedFd, EbpfError> {
        let fd = unsafe {
            syscall(
                SYS_PERF_EVENT_OPEN,
                &self.attrs,
                self.tid,
                self.cpu,
                self.group_id,
                0,
            )
        };
        if fd == -1 {
            let err = std::io::Error::last_os_error();
            Err(EbpfError::SyscallErr(format!(
                "open_perf_event failed: tid={}, cpu={}, group_id={}, err={}",
                self.tid, self.cpu, self.group_id, err
            )))
        } else {
            Ok(unsafe { OwnedFd::from_raw_fd(fd as RawFd) })
        }
    }
}

// 默认采样当前程序本身在所有cpu上的事件
impl Default for PerfEventFdBuilder {
    fn default() -> Self {
        let mut attrs = perf_event_attr {
            type_: PERF_TYPE_SOFTWARE,
            size: std::mem::size_of::<perf_event_attr>() as u32,
            config: PERF_COUNT_SW_CPU_CLOCK,
            ..Default::default()
        };
        attrs.__bindgen_anon_1.sample_period = DEFAULT_SAMPLE_PERIOD;
        attrs.set_disabled(1);
        attrs.set_exclude_kernel(1); // don't count time in kernel
        attrs.set_exclude_hv(1); // don't count time in hypervisor
        Self {
            attrs,
            cpu: 0,
            tid: 0,
            group_id: -1,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_builder_has_expected_values() {
        let builder = PerfEventFdBuilder::default();
        assert_eq!(builder.cpu, 0);
        assert_eq!(builder.tid, 0);
        assert_eq!(builder.group_id, -1);
    }

    #[test]
    fn attach_tid_updates_tid() {
        let mut builder = PerfEventFdBuilder::default();
        builder.attach_tid(12345);
        assert_eq!(builder.tid, 12345);
    }

    #[test]
    fn attach_cpu_updates_cpu() {
        let mut builder = PerfEventFdBuilder::default();
        builder.attach_cpu(3);
        assert_eq!(builder.cpu, 3);
    }

    #[test]
    fn default_attrs_type_is_software() {
        let builder = PerfEventFdBuilder::default();
        assert_eq!(builder.attrs.type_, PERF_TYPE_SOFTWARE);
    }

    #[test]
    fn default_attrs_config_is_cpu_clock() {
        let builder = PerfEventFdBuilder::default();
        assert_eq!(builder.attrs.config, PERF_COUNT_SW_CPU_CLOCK);
    }

    #[test]
    fn default_attrs_sample_period() {
        let builder = PerfEventFdBuilder::default();
        assert_eq!(
            unsafe { builder.attrs.__bindgen_anon_1.sample_period },
            DEFAULT_SAMPLE_PERIOD
        );
    }
}
