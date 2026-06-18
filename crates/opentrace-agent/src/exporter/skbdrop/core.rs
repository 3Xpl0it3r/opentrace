// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.

use prometheus::{IntCounterVec, Opts, Registry};

use opentrace_bpf::collectors::net::{SkbdropCollector, SkbdropConfig, SkbdropEvent};
use opentrace_bpf::sinks::EventSink;
use opentrace_bpf::types::net::{AddrV4, AddrV6};
use serde::Deserialize;
use tokio::sync::mpsc;

use crate::errors::AgntError;
use crate::sink::{KafkaRecord, LocalSinkCacheTask, SinkRecordSender, SseRecord};

use crate::exporter::{ExporterContext, ExporterRunner, ExporterSpec, run_collector};

use super::formatter::{SkbdropKafkaFormatter, SkbdropSseFormatter};

#[derive(Debug, Deserialize)]
pub struct SkbdropRequest {
    pub any_addr: Option<String>,
    pub src_addr: Option<String>,
    pub dst_addr: Option<String>,
    pub any_port: Option<u16>,
    pub src_port: Option<u16>,
    pub dst_port: Option<u16>,
    pub sink_name: Option<String>,
    pub watch: Option<bool>,
}

impl From<SkbdropRequest> for SkbdropConfig {
    fn from(value: SkbdropRequest) -> Self {
        Self {
            any_addr: value.any_addr.unwrap_or_default(),
            src_addr: value.src_addr.unwrap_or_default(),
            dst_addr: value.dst_addr.unwrap_or_default(),
            any_port: value.any_port.unwrap_or_default(),
            src_port: value.src_port.unwrap_or_default(),
            dst_port: value.dst_port.unwrap_or_default(),
            ..Default::default()
        }
    }
}

#[derive(Clone)]
pub(super) struct SkbdropMetrics {
    drops_total: IntCounterVec,
}

impl SkbdropMetrics {
    pub(super) fn new(registry: &Registry) -> Result<SkbdropMetrics, prometheus::Error> {
        let drops_total = IntCounterVec::new(
            Opts::new(
                "skbdrop",
                "Total number of skb drop events observed by skbdrop.",
            ),
            &["reason", "ip"],
        )?;
        registry.register(Box::new(drops_total.clone()))?;
        Ok(SkbdropMetrics { drops_total })
    }

    fn observe(&self, event: &SkbdropEvent) {
        let ip = event_source_ip(event);
        self.drops_total
            .with_label_values(&[event.drop_source_str(), ip.as_str()])
            .inc();
    }
}

fn event_source_ip(event: &SkbdropEvent) -> String {
    match event.l3_info.ip_version {
        4 => AddrV4::from(event.l3_info.saddr).to_string(),
        6 => AddrV6::from(event.l3_info.saddr).to_string(),
        _ => "0.0.0.0".to_string(),
    }
}

pub(super) struct SkbdropMetricSink {
    metrics: SkbdropMetrics,
}

impl SkbdropMetricSink {
    pub(super) fn new(metrics: SkbdropMetrics) -> Self {
        Self { metrics }
    }
}

impl EventSink<SkbdropEvent> for SkbdropMetricSink {
    fn dispatch(&mut self, event: SkbdropEvent) {
        self.metrics.observe(&event);
    }
}

pub struct SkbdropExporter;

impl SkbdropExporter {
    pub(crate) fn with_prometheus_metrics(
        config: SkbdropRequest,
    ) -> Result<ExporterSpec<impl ExporterRunner>, AgntError> {
        let registry = Registry::new();
        let metrics = SkbdropMetrics::new(&registry).map_err(AgntError::other)?;
        let sink = SkbdropMetricSink::new(metrics);

        Ok(ExporterSpec::new(
            Some(registry),
            None,
            move |context: ExporterContext| async move {
                let mut object = opentrace_bpf::open_object_storage();
                let mut collector = SkbdropCollector::new(&mut object, config.into(), sink)
                    .map_err(AgntError::other)?;

                run_collector(
                    &mut collector,
                    context.probe_registry,
                    context.interval,
                    context.cancel,
                )
                .await
            },
        ))
    }

    pub(crate) fn with_sink(
        config: SkbdropRequest,
        sink_record_sender: SinkRecordSender,
        sink_name: String,
    ) -> Result<ExporterSpec<impl ExporterRunner>, AgntError> {
        match sink_record_sender {
            SinkRecordSender::Kafka(kafka_sink) => Ok(ExporterSpec::new(
                None,
                Some(sink_name),
                move |context: ExporterContext| {
                    run_kafka_sink_exporter(config, kafka_sink, context)
                },
            )),
            SinkRecordSender::PrometheusPushGateway(_) => Err(AgntError::BadRequest(
                "PrometheusPGW sink not yet implemented".to_owned(),
            )),
        }
    }

    pub(crate) fn with_sse_sink(
        config: SkbdropRequest,
        sse_sink: mpsc::Sender<SseRecord>,
    ) -> ExporterSpec<impl ExporterRunner> {
        ExporterSpec::new(None, None, move |context: ExporterContext| {
            run_sse_sink_exporter(config, sse_sink, context)
        })
    }
}

async fn run_kafka_sink_exporter(
    config: SkbdropRequest,
    kafka_sink: mpsc::Sender<KafkaRecord>,
    context: ExporterContext,
) -> Result<(), AgntError> {
    let ExporterContext {
        probe_registry,
        interval,
        cancel,
    } = context;
    let cache_cancel = cancel.child_token();
    let (task, cache_sink) =
        LocalSinkCacheTask::new(kafka_sink, SkbdropKafkaFormatter::new, cache_cancel);

    let result = async {
        let mut object = opentrace_bpf::open_object_storage();
        let mut collector = SkbdropCollector::new(&mut object, config.into(), cache_sink)
            .map_err(AgntError::other)?;

        run_collector(&mut collector, probe_registry, interval, cancel.clone()).await
    }
    .await;
    cancel.cancel();

    task.stop().await;

    result
}

async fn run_sse_sink_exporter(
    config: SkbdropRequest,
    sse_sink: mpsc::Sender<SseRecord>,
    context: ExporterContext,
) -> Result<(), AgntError> {
    let ExporterContext {
        probe_registry,
        interval,
        cancel,
    } = context;
    let cache_cancel = cancel.child_token();
    let (task, cache_sink) =
        LocalSinkCacheTask::new(sse_sink, SkbdropSseFormatter::new, cache_cancel);

    let result = async {
        let mut object = opentrace_bpf::open_object_storage();
        let mut collector = SkbdropCollector::new(&mut object, config.into(), cache_sink)
            .map_err(AgntError::other)?;

        run_collector(&mut collector, probe_registry, interval, cancel.clone()).await
    }
    .await;
    cancel.cancel();

    task.stop().await;

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use opentrace_bpf::types::net::{Addr, L2Info, L3Info, L4Info};
    use prometheus::{Encoder, TextEncoder};

    fn skbdrop_event(src_ip: [u8; 4]) -> SkbdropEvent {
        SkbdropEvent {
            l2_info: L2Info { eth_proto: 0 },
            l3_info: L3Info {
                saddr: Addr {
                    v4addr: u32::from_ne_bytes(src_ip),
                },
                daddr: Addr { v4addr: 0 },
                tot_len: 0,
                ip_version: 4,
                l4_proto: 6,
            },
            l4_info: L4Info {
                sport: 0,
                dport: 0,
                tcpflags: 0,
            },
            stack_size: 0,
            stack: [0; 16],
            drop_reason: 0,
            drop_source: 1,
        }
    }

    #[test]
    fn skbdrop_metric_uses_reason_ip_and_count() {
        let registry = Registry::new();
        let metrics = SkbdropMetrics::new(&registry).unwrap();
        let event = skbdrop_event([10, 0, 0, 1]);

        metrics.observe(&event);
        metrics.observe(&event);

        let mut output = Vec::new();
        TextEncoder::new()
            .encode(&registry.gather(), &mut output)
            .unwrap();
        let output = String::from_utf8(output).unwrap();

        assert!(output.contains("skbdrop{ip=\"10.0.0.1\",reason=\"kfree_skb\"} 2"));
    }
}
