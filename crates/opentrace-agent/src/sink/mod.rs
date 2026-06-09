// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.
mod build;

use std::marker::PhantomData;

use opentrace_bpf::sinks::EventSink;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum SinkConfig {
    Kafka(KafkaConfig),
    PrometheusPGW(PrometheusPushGWConfig),
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct KafkaConfig {}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PrometheusPushGWConfig {}

pub struct KafkaSink<T> {
    _phantom: PhantomData<T>,
}

impl<T> KafkaSink<T> {
    pub fn new() -> Self {
        Self {
            _phantom: PhantomData,
        }
    }
}

impl<T> EventSink<T> for KafkaSink<T> {
    fn dispatch(&mut self, event: T) {
        todo!()
    }
}
