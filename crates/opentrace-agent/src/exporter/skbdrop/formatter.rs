// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.
use std::{io, mem};

use opentrace_bpf::collectors::net::SkbdropEvent;
use opentrace_bpf::format::StructeredFormatter;
use opentrace_bpf::symbolizers::{Source, SymbolizeInput, SymbolizerProvider};
use serde_json::{Value, json};

use crate::sink::{KafkaRecord, SseRecord};

pub(super) struct SkbdropKafkaFormatter {
    symbolizer_provider: SymbolizerProvider<'static>,
}

impl SkbdropKafkaFormatter {
    pub(super) fn new() -> Self {
        Self {
            symbolizer_provider: SymbolizerProvider::default(),
        }
    }
}

impl StructeredFormatter<SkbdropEvent> for SkbdropKafkaFormatter {
    type Output = KafkaRecord;

    fn format(&self, event: SkbdropEvent) -> Result<Self::Output, io::Error> {
        let raw_event = event_with_symbolized_stack(&self.symbolizer_provider, event)?;
        let mut value = serde_json::to_vec(&json!({
            "program": "skbdrop",
            "raw_event": raw_event,
        }))
        .map_err(io::Error::other)?;
        value.push(b'\n');
        Ok(KafkaRecord::new(value))
    }
}

pub(super) struct SkbdropSseFormatter {
    symbolizer_provider: SymbolizerProvider<'static>,
}

impl SkbdropSseFormatter {
    pub(super) fn new() -> Self {
        Self {
            symbolizer_provider: SymbolizerProvider::default(),
        }
    }
}

impl StructeredFormatter<SkbdropEvent> for SkbdropSseFormatter {
    type Output = SseRecord;

    fn format(&self, event: SkbdropEvent) -> Result<Self::Output, io::Error> {
        let raw_event = event_with_symbolized_stack(&self.symbolizer_provider, event)?;
        let data = serde_json::to_string(&raw_event).map_err(io::Error::other)?;
        Ok(SseRecord::new("skbdrop", data))
    }
}

fn event_with_symbolized_stack(
    symbolizer_provider: &SymbolizerProvider<'_>,
    event: SkbdropEvent,
) -> Result<Value, io::Error> {
    let source = Source::Kernel;
    let stack = resolve_stack_names(symbolizer_provider, &source, &event);
    let mut value = serde_json::to_value(event).map_err(io::Error::other)?;

    if !stack.is_empty() {
        if let Value::Object(fields) = &mut value {
            fields.insert(
                "stack".to_string(),
                Value::Array(stack.into_iter().map(Value::String).collect()),
            );
        }
    }

    Ok(value)
}

fn resolve_stack_names(
    symbolizer_provider: &SymbolizerProvider<'_>,
    source: &Source<'_>,
    event: &SkbdropEvent,
) -> Vec<String> {
    let stack_len = effective_stack_len(event);
    if stack_len == 0 {
        return Vec::new();
    }

    let symbolizer = symbolizer_provider.get_symbolizer(source);
    event.stack[..stack_len]
        .iter()
        .map(|addr| {
            symbolizer
                .resolve(SymbolizeInput {
                    source: source.clone(),
                    addr: *addr,
                })
                .name
                .into_owned()
        })
        .collect()
}

fn effective_stack_len(event: &SkbdropEvent) -> usize {
    if event.stack_size <= 0 {
        return 0;
    }

    ((event.stack_size as usize) / mem::size_of::<u64>()).min(event.stack.len())
}
