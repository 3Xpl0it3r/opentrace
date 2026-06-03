// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.

use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};

use crate::EbpfError;

const AVAILABLE_FILTER_FUNCTIONS: &str = "/sys/kernel/debug/tracing/available_filter_functions";
const AVAILABLE_EVENTS: &str = "/sys/kernel/debug/tracing/available_events";

#[derive(Clone)]
pub struct Registry {
    kprobes: HashMap<String, bool>,
    tracepoints: HashMap<String, bool>,
}

impl Registry {
    pub fn try_init() -> Result<Self, EbpfError> {
        Ok(Self {
            kprobes: read_kprobes()?,
            tracepoints: read_tracepoints()?,
        })
    }

    #[inline]
    pub(crate) fn kprobe_is_available(&self, kprobe: &str) -> bool {
        let supported = self.kprobes.contains_key(kprobe);
        if !supported {
            println!("kprobe {} is not supported in current os", kprobe);
        }
        supported
    }

    #[inline]
    pub(crate) fn tracepoint_is_available(&self, tp: &str) -> bool {
        let supported = self.tracepoints.contains_key(tp);
        if !supported {
            println!("tracepoint {} is not supported in current os", tp);
        }
        supported
    }
}

#[inline]
fn read_kprobes() -> Result<HashMap<String, bool>, EbpfError> {
    read_kprobes_from(AVAILABLE_FILTER_FUNCTIONS)
}

#[inline]
fn read_kprobes_from(path: &str) -> Result<HashMap<String, bool>, EbpfError> {
    let file = File::open(path).map_err(EbpfError::IO)?;
    let reader = BufReader::new(file);
    reader
        .lines()
        .try_fold(HashMap::new(), |mut functions, line| {
            let line = line.map_err(EbpfError::IO)?;
            let func_name = line.trim();
            if !func_name.is_empty() {
                functions.insert(func_name.to_string(), true);
            }
            Ok(functions)
        })
}

#[inline]
fn read_tracepoints() -> Result<HashMap<String, bool>, EbpfError> {
    read_tracepoints_from(AVAILABLE_EVENTS)
}

#[inline]
fn read_tracepoints_from(path: &str) -> Result<HashMap<String, bool>, EbpfError> {
    let file = File::open(path).map_err(EbpfError::IO)?;
    let reader = BufReader::new(file);
    reader
        .lines()
        .try_fold(HashMap::new(), |mut trace_map, line| {
            let line = line.map_err(EbpfError::IO)?;
            let tracepoint = line.trim();
            // 跳过空行和不符合格式的行
            if !tracepoint.is_empty() && tracepoint.contains(':') {
                trace_map.insert(tracepoint.to_string(), true);
            }
            Ok(trace_map)
        })
}

#[cfg(test)]
mod tests {
    use super::{Registry, read_kprobes_from, read_tracepoints_from};

    const DATA_DIR: &str = "tests/data";

    #[test]
    fn reads_available_filter_functions_fixture() {
        let path = format!("{DATA_DIR}/available_filter_functions");
        let kprobes = read_kprobes_from(&path).unwrap();

        assert!(kprobes.contains_key("do_one_initcall"));
        assert!(kprobes.contains_key("x86_pmu_enable_event"));
        assert!(!kprobes.contains_key(""));
    }

    #[test]
    fn reads_available_events_fixture() {
        let path = format!("{DATA_DIR}/available_events");
        let tracepoints = read_tracepoints_from(&path).unwrap();

        assert!(tracepoints.contains_key("syscalls:sys_enter_arch_prctl"));
        assert!(tracepoints.contains_key("irq_vectors:spurious_apic_exit"));
        assert!(!tracepoints.contains_key("malformed_tracepoint_without_group"));
    }

    #[test]
    fn registry_checks_loaded_probe_availability() {
        let kprobes = read_kprobes_from(&format!("{DATA_DIR}/available_filter_functions")).unwrap();
        let tracepoints = read_tracepoints_from(&format!("{DATA_DIR}/available_events")).unwrap();
        let registry = Registry {
            kprobes,
            tracepoints,
        };

        assert!(registry.kprobe_is_available("do_one_initcall"));
        assert!(!registry.kprobe_is_available("definitely_missing_probe"));
        assert!(registry.tracepoint_is_available("syscalls:sys_enter_arch_prctl"));
        assert!(!registry.tracepoint_is_available("missing:tracepoint"));
    }
}
