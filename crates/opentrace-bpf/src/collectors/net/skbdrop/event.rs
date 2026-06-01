use std::mem;

use serde::ser::SerializeStruct as _;
use serde::{Deserialize, Serialize};

use crate::types::net::{L2Info, L3Info, L4Info};

/// drop 事件来源，与 skbdrop.bpf.c 中的 DROP_SRC_* 对齐。
pub const DROP_SRC_KFREE_SKB: u8 = 1;
pub const DROP_SRC_NF_HOOK: u8 = 2;

// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct Event {
    pub l2_info: L2Info,
    pub l3_info: L3Info,
    pub l4_info: L4Info,
    /* pub pkt_info: PktInfo, */
    pub stack_size: i64,
    pub stack: [u64; 16],
    drop_reason: u8,
    drop_source: u8,
}

impl Event {
    /// 将原始的 drop_source 数值翻译成可读字符串。
    pub fn drop_source_str(&self) -> &'static str {
        match self.drop_source {
            DROP_SRC_KFREE_SKB => "kfree_skb",
            DROP_SRC_NF_HOOK => "nf_hook(drop/reject)",
            _ => "unknown",
        }
    }

    /// 实际有效的栈帧数（按字节 size / sizeof(u64)，并被数组容量截断）。
    fn effective_stack_len(&self) -> usize {
        if self.stack_size <= 0 {
            return 0;
        }
        ((self.stack_size as usize) / mem::size_of::<u64>()).min(self.stack.len())
    }
}

impl Serialize for Event {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let stack_len = self.effective_stack_len();
        // 基础字段: l2, l3, l4, source；有栈时额外多一项 stack
        let field_cnt = if stack_len > 0 { 5 } else { 4 };

        let mut state = serializer.serialize_struct("Event", field_cnt)?;
        state.serialize_field("l2", &self.l2_info)?;
        state.serialize_field("l3", &self.l3_info)?;
        state.serialize_field("l4", &self.l4_info)?;
        state.serialize_field("source", self.drop_source_str())?;
        if stack_len > 0 {
            state.serialize_field("stack", &self.stack[..stack_len])?;
        }
        state.end()
    }
}

impl<'de> Deserialize<'de> for Event {
    fn deserialize<D>(_deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        todo!()
    }
}
