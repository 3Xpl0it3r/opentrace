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
    pub drop_reason: u8,
    pub drop_source: u8,
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

#[cfg(test)]
mod tests {
    use super::{DROP_SRC_KFREE_SKB, DROP_SRC_NF_HOOK, Event};
    use crate::types::net::{Addr, L2Info, L3Info, L4Info};

    fn event(stack_size: i64) -> Event {
        Event {
            l2_info: L2Info { eth_proto: 0 },
            l3_info: L3Info {
                saddr: Addr { v4addr: 0 },
                daddr: Addr { v4addr: 0 },
                tot_len: 0,
                ip_version: 4,
                l4_proto: 6,
            },
            l4_info: L4Info {
                sport: 0,
                dport: 0,
                tcpflags: 0,
            },
            stack_size,
            stack: [0; 16],
            drop_reason: 0,
            drop_source: DROP_SRC_KFREE_SKB,
        }
    }

    #[test]
    fn translates_drop_source() {
        let mut event = event(0);
        assert_eq!(event.drop_source_str(), "kfree_skb");

        event.drop_source = DROP_SRC_NF_HOOK;
        assert_eq!(event.drop_source_str(), "nf_hook(drop/reject)");

        event.drop_source = 99;
        assert_eq!(event.drop_source_str(), "unknown");
    }

    #[test]
    fn serializes_stack_when_stack_size_is_positive() {
        let mut event = event(16);
        event.stack[0] = 1;
        event.stack[1] = 2;

        let value = serde_json::to_value(event).unwrap();

        assert_eq!(value["source"], "kfree_skb");
        assert_eq!(value["stack"], serde_json::json!([1, 2]));
    }

    #[test]
    fn omits_stack_when_stack_size_is_not_positive() {
        let value = serde_json::to_value(event(0)).unwrap();

        assert!(value.get("stack").is_none());
    }
}
