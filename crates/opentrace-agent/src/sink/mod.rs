// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.
mod cache;
mod kafka;
mod prometheus;
mod manager;

pub(crate) use cache::{SinkCacheTask, SinkCacher};
pub(crate) use kafka::KafkaRecord;
pub use manager::SinkConfig;
pub(crate) use manager::SinkManager;
pub use manager::{SinkRecordReceiver, SinkRecordSender};
