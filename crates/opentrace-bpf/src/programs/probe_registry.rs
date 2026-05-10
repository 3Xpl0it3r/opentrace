// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.

use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};

use crate::EbpfError;

const AVAILABLE_FILTER_FUNCTIONS: &str = "/sys/kernel/debug/tracing/available_filter_functions";
const AVAILABLE_EVENTS: &str = "/sys/kernel/debug/tracing/available_events";

#[derive(Clone)]
pub struct ProbeRegistry {
    kprobes: HashMap<String, bool>,
    tracepoints: HashMap<String, bool>,
}

impl ProbeRegistry {
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
    let file = File::open(AVAILABLE_FILTER_FUNCTIONS).map_err(EbpfError::IO)?;
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
    let file = File::open(AVAILABLE_EVENTS).map_err(EbpfError::IO)?;
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
