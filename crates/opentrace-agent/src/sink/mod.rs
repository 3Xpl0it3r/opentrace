// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.
mod cache;
mod kafka;
mod prometheus;
mod manager;
mod sse;

pub(crate) use cache::{LocalSinkCacheTask, SinkCacheTask, SinkCacher};
pub(crate) use kafka::{KafkaRecord, KafkaSink};
pub use manager::SinkConfig;
pub(crate) use manager::SinkManager;
pub use manager::{SinkRecordReceiver, SinkRecordSender};
pub(crate) use sse::SseRecord;
