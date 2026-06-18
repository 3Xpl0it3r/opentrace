use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use crate::errors::AgntError;

// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PrometheusConfig {}

#[derive(Debug)]
pub(crate) struct PrometheusRecord {
    pub(crate) body: Vec<u8>,
    pub(crate) content_type: &'static str,
}

// prometheus pushgateway sink
pub struct PrometheusSink {
    _config: PrometheusConfig,
}

impl PrometheusSink {
    pub fn new(config: PrometheusConfig) -> Result<Self, AgntError> {
        //  真正的去创建一个httpclient
        Ok(Self { _config: config })
    }

    pub async fn run(
        mut self,
        mut rx: mpsc::Receiver<PrometheusRecord>,
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

    async fn send(&mut self, record: PrometheusRecord) -> Result<(), AgntError> {
        let PrometheusRecord { body, content_type } = record;
        drop((body, content_type));
        Ok(())
    }
}
