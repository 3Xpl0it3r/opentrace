// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.
use std::io;

use opentrace_bpf::collectors::net::SkbdropEvent;
use opentrace_bpf::format::StructeredFormatter;

use crate::sink::KafkaRecord;

// kafka formatter

pub(super) struct SkbdropKafkaFormatter;

impl StructeredFormatter<SkbdropEvent> for SkbdropKafkaFormatter {
    type Output = KafkaRecord;

    fn format(&self, event: SkbdropEvent) -> Result<Self::Output, io::Error> {
        let mut value = serde_json::to_vec(&event).map_err(io::Error::other)?;
        value.push(b'\n');
        Ok(KafkaRecord::new("skbdrop".into(), value))
    }
}
