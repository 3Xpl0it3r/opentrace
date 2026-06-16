use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use crate::errors::AgntError;

#[derive(Debug)]
pub(crate) struct KafkaRecord {
    pub(crate) topic: String,
    pub(crate) value: Vec<u8>,
}

impl KafkaRecord {
    pub(crate) fn new(topic: String, value: Vec<u8>) -> Self {
        Self { topic, value }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct KafkaConfig {}

pub struct KafkaSink {
    _config: KafkaConfig,
}

impl KafkaSink {
    pub fn new(config: KafkaConfig) -> Self {
        // todo 真正的去创建kafka client
        Self { _config: config }
    }

    pub async fn run(
        mut self,
        mut rx: mpsc::Receiver<KafkaRecord>,
        cancel: CancellationToken,
    ) -> Result<(), AgntError> {
        loop {
            tokio::select! {
                _ = cancel.cancelled() => {
                    while let Ok(record) = rx.try_recv() {
                        self.send(record).await?;
                    }
                    break;
                }
                record = rx.recv() => {
                    let Some(record) = record else {
                        break;
                    };
                    self.send(record).await?;
                }
            }
        }

        Ok(())
    }

    async fn send(&mut self, record: KafkaRecord) -> Result<(), AgntError> {
        let KafkaRecord { topic: key, value } = record;
        println!("send record to kafka");
        drop((key, value));
        Ok(())
    }
}
