use criterion::{Criterion, black_box, criterion_group, criterion_main};
use opentrace_bpf::exporter::{
    Exporter as _, SimpleBoundChannelExpoter, SimpleUnboundChannelExporter, StreamWriterExpoter,
};
use opentrace_bpf::format::StreamFormatter;
use std::io;

struct TestFormatter;

impl StreamFormatter<u32> for TestFormatter {
    fn format<W: io::Write>(&self, w: &mut W, value: &u32) -> io::Result<()> {
        write!(w, "value={value}")
    }
}

struct FailingFormatter;

impl StreamFormatter<u32> for FailingFormatter {
    fn format<W: io::Write>(&self, _w: &mut W, _value: &u32) -> io::Result<()> {
        Err(io::Error::other("format failed"))
    }
}

fn bench_stream_writer_exporter(c: &mut Criterion) {
    let mut exporter = StreamWriterExpoter::new(Vec::new(), TestFormatter);

    c.bench_function("stream_writer_exporter_dispatch", |b| {
        b.iter(|| {
            exporter.dispatch(black_box(42));
        })
    });
}

fn bench_stream_writer_exporter_failing(c: &mut Criterion) {
    let mut exporter = StreamWriterExpoter::new(Vec::new(), FailingFormatter);

    c.bench_function("stream_writer_exporter_dispatch_failing", |b| {
        b.iter(|| {
            exporter.dispatch(black_box(42));
        })
    });
}

fn bench_unbounded_channel_exporter(c: &mut Criterion) {
    let (mut exporter, _rx) = SimpleUnboundChannelExporter::<String, &str>::new();

    c.bench_function("unbounded_channel_exporter_dispatch", |b| {
        b.iter(|| {
            exporter.dispatch(black_box("hello"));
        })
    });
}

fn bench_bounded_channel_exporter_creation(c: &mut Criterion) {
    c.bench_function("bounded_channel_exporter_create_capacity_8", |b| {
        b.iter(|| {
            let (_exporter, _rx) = SimpleBoundChannelExpoter::<u32>::new(black_box(8));
        })
    });

    c.bench_function("bounded_channel_exporter_create_capacity_1024", |b| {
        b.iter(|| {
            let (_exporter, _rx) = SimpleBoundChannelExpoter::<u32>::new(black_box(1024));
        })
    });
}

criterion_group!(
    benches,
    bench_stream_writer_exporter,
    bench_stream_writer_exporter_failing,
    bench_unbounded_channel_exporter,
    bench_bounded_channel_exporter_creation
);
criterion_main!(benches);
