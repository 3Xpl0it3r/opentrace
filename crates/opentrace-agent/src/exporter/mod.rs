// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.
mod task;
mod skbdrop;

pub(super) type CollectorBuildeFn = dyn FnOnce(
        opentrace_bpf::CollectorObject,
    ) -> Result<
        (
            opentrace_bpf::CollectorObject,
            Box<dyn opentrace_bpf::collectors::Collector + 'static>,
        ),
        opentrace_bpf::EbpfError,
    > + Send;

pub(crate) use skbdrop::SkbCollectorBuilder;
pub(crate) use task::{Exporter, Task as ExporterTask};
