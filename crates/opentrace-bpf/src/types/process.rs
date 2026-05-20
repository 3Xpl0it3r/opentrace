// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.

use serde::ser::SerializeStruct;
use serde::{Deserialize, Serialize};

use crate::utils;

// ProcessInfo[#TODO] (shoule add some comments )
#[derive(Clone, Copy)]
#[repr(C)]
pub struct ProcessInfo {
    pub pid: u32,
    pub tgid: u32,
    pub comm: [u8; 16],
}

impl Serialize for ProcessInfo {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut state = serializer.serialize_struct("process_info", 3)?;
        state.serialize_field("pid", &self.pid)?;
        state.serialize_field("tgid", &self.tgid)?;
        state.serialize_field("comm", &utils::cstring::from_bytes_lossy(&self.comm))?;
        state.end()
    }
}

impl<'de> Deserialize<'de> for ProcessInfo {
    fn deserialize<D>(_deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        todo!()
    }
}
