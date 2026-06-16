// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.

use prometheus::{IntCounterVec, Opts, Registry};

use opentrace_bpf::collectors::net::{SkbdropCollector, SkbdropConfig, SkbdropEvent};
use opentrace_bpf::sinks::EventSink;
use serde::Deserialize;
use tokio::sync::mpsc;

use crate::errors::AgntError;
use crate::sink::{KafkaRecord, SinkCacheTask, SinkCacher, SinkRecordSender};

use crate::exporter::{ExporterContext, ExporterRunner, ExporterSpec, run_collector};

use super::formatter::SkbdropKafkaFormatter;

#[derive(Debug, Deserialize)]
pub struct SkbdropRequest {
    pub any_addr: Option<String>,
    pub src_addr: Option<String>,
    pub dst_addr: Option<String>,
    pub any_port: Option<u16>,
    pub src_port: Option<u16>,
    pub dst_port: Option<u16>,
    pub sink_name: Option<String>,
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
                "opentrace_skbdrop_drops_total",
                "Total number of skb drop events observed by opentrace.",
            ),
            &["source", "ip_version", "l4_proto", "drop_reason"],
        )?;
        registry.register(Box::new(drops_total.clone()))?;
        Ok(SkbdropMetrics { drops_total })
    }

    fn observe(&self, event: &SkbdropEvent) {
        let ip_version = event.l3_info.ip_version.to_string();
        let l4_proto = event.l3_info.l4_proto.to_string();
        let drop_reason = event.drop_reason.to_string();
        self.drops_total
            .with_label_values(&[
                event.drop_source_str(),
                ip_version.as_str(),
                l4_proto.as_str(),
                drop_reason.as_str(),
            ])
            .inc();
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
    let (cacher, cache_sink) = SinkCacher::new(kafka_sink, SkbdropKafkaFormatter);
    let task = SinkCacheTask::new(cacher, cache_cancel);

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
