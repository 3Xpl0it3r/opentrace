use std::net::{TcpStream, ToSocketAddrs};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use rdkafka::ClientConfig;
use rdkafka::producer::{FutureProducer, FutureRecord, Producer};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use crate::errors::AgntError;

const KAFKA_CONNECTION_CHECK_TIMEOUT: Duration = Duration::from_secs(5);
const KAFKA_TCP_CHECK_TIMEOUT: Duration = Duration::from_secs(2);
const KAFKA_SEND_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Debug)]
pub(crate) struct KafkaRecord {
    pub(crate) value: Vec<u8>,
}

impl KafkaRecord {
    pub(crate) fn new(value: Vec<u8>) -> Self {
        Self { value }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct KafkaConfig {
    #[serde(default, alias = "bootstrap")]
    pub brokers: Vec<String>,
    pub topic: String,
}

pub struct KafkaSink {
    config: KafkaConfig,
    client: FutureProducer,
}

impl KafkaSink {
    pub fn new(config: KafkaConfig) -> Result<Self, AgntError> {
        let client = create_checked_client(&config)?;
        Ok(Self { config, client })
    }

    pub(crate) fn send_debug(config: KafkaConfig) -> Result<(), AgntError> {
        let client = create_checked_client(&config)?;
        send_debug_record(&client, &config.topic)
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
        let KafkaRecord { value } = record;
        let record = FutureRecord::<(), _>::to(&self.config.topic).payload(&value);

        match self.client.send(record, KAFKA_SEND_TIMEOUT).await {
            Ok(_) => Ok(()),
            Err((e, _)) => Err(AgntError::Internal(format!(
                "send kafka record failed: {e}"
            ))),
        }
    }
}

fn create_checked_client(config: &KafkaConfig) -> Result<FutureProducer, AgntError> {
    let brokers = normalized_brokers(config)?;
    if brokers.is_empty() {
        return Err(AgntError::BadRequest(
            "kafka sink requires at least one broker".to_string(),
        ));
    }
    if config.topic.trim().is_empty() {
        return Err(AgntError::BadRequest(
            "kafka sink requires topic".to_string(),
        ));
    }

    check_brokers_reachable(&brokers)?;

    let client: FutureProducer = ClientConfig::new()
        .set("bootstrap.servers", brokers.join(","))
        .set(
            "message.timeout.ms",
            KAFKA_SEND_TIMEOUT.as_millis().to_string(),
        )
        .create()
        .map_err(|e| AgntError::Internal(format!("create kafka client failed: {e}")))?;

    let metadata = client
        .client()
        .fetch_metadata(None, KAFKA_CONNECTION_CHECK_TIMEOUT)
        .map_err(|e| {
            AgntError::Internal(format!(
                "kafka is unavailable, brokers='{}': {e}",
                brokers.join(",")
            ))
        })?;
    if metadata.brokers().is_empty() {
        return Err(AgntError::Internal(format!(
            "kafka is unavailable, brokers='{}': no brokers returned from metadata",
            brokers.join(",")
        )));
    }

    Ok(client)
}

fn send_debug_record(client: &FutureProducer, topic: &str) -> Result<(), AgntError> {
    let value = debug_record_value()?;
    let record = FutureRecord::<(), _>::to(topic).payload(&value);
    let delivery_future = client
        .send_result(record)
        .map_err(|(e, _)| AgntError::Internal(format!("submit kafka debug record failed: {e}")))?;

    futures::executor::block_on(delivery_future)
        .map_err(|e| AgntError::Internal(format!("wait kafka debug delivery failed: {e}")))?
        .map(|_| ())
        .map_err(|(e, _)| AgntError::Internal(format!("deliver kafka debug record failed: {e}")))
}

fn debug_record_value() -> Result<Vec<u8>, AgntError> {
    let message = format!("this is debug {}", current_time_millis()?);
    let mut value = serde_json::to_vec(&json!({
        "program": "test",
        "message": message,
    }))
    .map_err(|e| AgntError::Internal(format!("serialize kafka debug record failed: {e}")))?;
    value.push(b'\n');
    Ok(value)
}

fn current_time_millis() -> Result<u128, AgntError> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .map_err(|e| AgntError::other(format!("system clock before unix epoch: {e}")))
}

fn normalized_brokers(config: &KafkaConfig) -> Result<Vec<String>, AgntError> {
    let brokers: Vec<_> = config
        .brokers
        .iter()
        .map(|broker| broker.trim())
        .filter(|broker| !broker.is_empty())
        .map(ToOwned::to_owned)
        .collect();
    if brokers.len() != config.brokers.len() {
        return Err(AgntError::BadRequest(
            "kafka sink brokers cannot contain empty entries".to_string(),
        ));
    }
    Ok(brokers)
}

fn check_brokers_reachable(brokers: &[String]) -> Result<(), AgntError> {
    let mut errors = Vec::new();

    for broker in brokers {
        let broker_addr = broker_address_for_tcp_check(broker);
        match broker_addr.to_socket_addrs() {
            Ok(addrs) => {
                let mut resolved = false;
                for addr in addrs {
                    resolved = true;
                    if TcpStream::connect_timeout(&addr, KAFKA_TCP_CHECK_TIMEOUT).is_ok() {
                        return Ok(());
                    }
                }
                if !resolved {
                    errors.push(format!("{broker}: no socket addresses resolved"));
                } else {
                    errors.push(format!(
                        "{broker}: connection failed within {}s",
                        KAFKA_TCP_CHECK_TIMEOUT.as_secs()
                    ));
                }
            }
            Err(e) => errors.push(format!("{broker}: {e}")),
        }
    }

    Err(AgntError::Internal(format!(
        "kafka is unavailable, brokers='{}': {}",
        brokers.join(","),
        errors.join("; ")
    )))
}

fn broker_address_for_tcp_check(broker: &str) -> String {
    let broker = broker
        .split_once("://")
        .map(|(_, broker)| broker)
        .unwrap_or(broker);
    if broker.rsplit_once(':').is_some() || broker.starts_with('[') {
        broker.to_owned()
    } else {
        format!("{broker}:9092")
    }
}
