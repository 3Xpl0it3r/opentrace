// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.

use std::sync::Arc;

use prometheus::{IntCounterVec, Opts, Registry};

use opentrace_bpf::collectors::net::{SkbdropCollector, SkbdropConfig, SkbdropEvent};
use opentrace_bpf::sinks::EventSink;
use serde::Deserialize;

use crate::errors::AgntError;
use crate::sink::KafkaSink;

use super::Exporter;
use super::helper::build_exporter;

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

pub struct SkbCollectorBuilder;

impl SkbCollectorBuilder {
    pub(crate) fn prepare_prometheus(config: SkbdropRequest) -> Result<Arc<Exporter>, AgntError> {
        let registry = Registry::new();
        let metrics = SkbdropMetrics::new(&registry).map_err(AgntError::other)?;
        let sink = SkbdropMetricSink::new(metrics);

        build_exporter(Some(registry), move |exporter, probe_registry, interval| {
            Box::pin(async move {
                let mut object = opentrace_bpf::open_object_storage();
                let mut collector = SkbdropCollector::new(&mut object, config.into(), sink)
                    .map_err(AgntError::other)?;

                super::core::run(&exporter, &mut collector, probe_registry, interval).await
            })
        })
    }

    pub(crate) fn prepare_kafka(config: SkbdropRequest) -> Result<Arc<Exporter>, AgntError> {
        let sink = KafkaSink::<SkbdropEvent>::new();

        build_exporter(None, move |exporter, probe_registry, interval| {
            Box::pin(async move {
                let mut object = opentrace_bpf::open_object_storage();
                let mut collector = SkbdropCollector::new(&mut object, config.into(), sink)
                    .map_err(AgntError::other)?;

                super::core::run(&exporter, &mut collector, probe_registry, interval).await
            })
        })
    }
}
