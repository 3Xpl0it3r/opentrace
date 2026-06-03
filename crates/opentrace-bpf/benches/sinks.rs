use criterion::{Criterion, black_box, criterion_group, criterion_main};
use opentrace_bpf::format::StreamFormatter;
use opentrace_bpf::sinks::{
    BoundedChannelSink, EventSink as _, StreamWriterSink, UnboundedChannelSink,
};
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

fn bench_stream_writer_sink(c: &mut Criterion) {
    let mut sink = StreamWriterSink::new(Vec::new(), TestFormatter);

    c.bench_function("stream_writer_sink_dispatch", |b| {
        b.iter(|| {
            sink.dispatch(black_box(42));
        })
    });
}

fn bench_stream_writer_sink_failing(c: &mut Criterion) {
    let mut sink = StreamWriterSink::new(Vec::new(), FailingFormatter);

    c.bench_function("stream_writer_sink_dispatch_failing", |b| {
        b.iter(|| {
            sink.dispatch(black_box(42));
        })
    });
}

fn bench_unbounded_channel_sink(c: &mut Criterion) {
    let (mut sink, _rx) = UnboundedChannelSink::<String, &str>::new();

    c.bench_function("unbounded_channel_sink_dispatch", |b| {
        b.iter(|| {
            sink.dispatch(black_box("hello"));
        })
    });
}

fn bench_bounded_channel_sink_creation(c: &mut Criterion) {
    c.bench_function("bounded_channel_sink_create_capacity_8", |b| {
        b.iter(|| {
            let (_sink, _rx) = BoundedChannelSink::<u32>::new(black_box(8));
        })
    });

    c.bench_function("bounded_channel_sink_create_capacity_1024", |b| {
        b.iter(|| {
            let (_sink, _rx) = BoundedChannelSink::<u32>::new(black_box(1024));
        })
    });
}

criterion_group!(
    benches,
    bench_stream_writer_sink,
    bench_stream_writer_sink_failing,
    bench_unbounded_channel_sink,
    bench_bounded_channel_sink_creation
);
criterion_main!(benches);
