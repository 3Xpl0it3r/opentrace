// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.

#[derive(Debug)]
pub(crate) struct SseRecord {
    event: &'static str,
    data: String,
}

impl SseRecord {
    pub(crate) fn new(event: &'static str, data: String) -> Self {
        Self { event, data }
    }

    pub(crate) fn event(&self) -> &'static str {
        self.event
    }

    pub(crate) fn data(&self) -> &str {
        &self.data
    }
}
