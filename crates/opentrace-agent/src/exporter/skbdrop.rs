// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.

use std::sync::Arc;

use prometheus::{IntCounterVec, Opts, Registry};

use opentrace_bpf::collectors::net::{SkbdropCollector, SkbdropConfig, SkbdropEvent};
use opentrace_bpf::sinks::EventSink;

use crate::errors::AgntError;

use super::Exporter;

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
    pub(crate) fn prepare(config: SkbdropConfig) -> Result<Arc<Exporter>, AgntError> {
        let registry = Registry::new();
        let metrics =
            SkbdropMetrics::new(&registry).map_err(|err| AgntError::Other(err.to_string()))?;
        let sink = SkbdropMetricSink::new(metrics);
        let exporter = Exporter::new(registry);

        let builder = move |mut object| {
            let collector = SkbdropCollector::new(&mut object, config, sink)?;
            let collector: Box<dyn opentrace_bpf::collectors::Collector> = Box::new(collector);
            let collector: Box<dyn opentrace_bpf::collectors::Collector + 'static> =
                // SAFETY: Collector trait is 'static, transmute extends lifetime for storage
                unsafe { std::mem::transmute(collector) };
            Ok((object, collector))
        };

        exporter.set_builder(Box::new(builder));

        Ok(Arc::new(exporter))
    }
}
