use criterion::{Criterion, black_box, criterion_group, criterion_main};
use opentrace_bpf::utils::cstring;
use opentrace_bpf::utils::net::{ipaddr_to_u128, tcp_flags, u32_to_ipaddr_v4, u128_to_ipaddr_v6};
use opentrace_bpf::utils::units::bytes::Bytes;
use opentrace_bpf::utils::units::time::{Nanoseconds, TimeAsNanosecond};

fn bench_bytes_display(c: &mut Criterion) {
    c.bench_function("bytes_display_0", |b| {
        b.iter(|| Bytes(black_box(0)).to_string())
    });

    c.bench_function("bytes_display_1023", |b| {
        b.iter(|| Bytes(black_box(1023)).to_string())
    });

    c.bench_function("bytes_display_1k", |b| {
        b.iter(|| Bytes(black_box(1024)).to_string())
    });

    c.bench_function("bytes_display_1m", |b| {
        b.iter(|| Bytes(black_box(1024 * 1024)).to_string())
    });
}

fn bench_cstring_from_bytes_lossy(c: &mut Criterion) {
    let data_with_nul = b"eth0\0ignored";
    let data_without_nul = b"eth0";
    let data_invalid_utf8 = &[0xff, 0x00];

    c.bench_function("cstring_from_bytes_lossy_with_nul", |b| {
        b.iter(|| cstring::from_bytes_lossy(black_box(data_with_nul)))
    });

    c.bench_function("cstring_from_bytes_lossy_without_nul", |b| {
        b.iter(|| cstring::from_bytes_lossy(black_box(data_without_nul)))
    });

    c.bench_function("cstring_from_bytes_lossy_invalid_utf8", |b| {
        b.iter(|| cstring::from_bytes_lossy(black_box(data_invalid_utf8)))
    });
}

fn bench_net_u32_to_ipaddr_v4(c: &mut Criterion) {
    let addr = u32::from_ne_bytes([127, 0, 0, 1]);

    c.bench_function("u32_to_ipaddr_v4", |b| {
        b.iter(|| u32_to_ipaddr_v4(black_box(addr)))
    });
}

fn bench_net_u128_to_ipaddr_v6(c: &mut Criterion) {
    c.bench_function("u128_to_ipaddr_v6", |b| {
        b.iter(|| u128_to_ipaddr_v6(black_box(1)))
    });
}

fn bench_net_ipaddr_to_u128(c: &mut Criterion) {
    c.bench_function("ipaddr_to_u128_empty", |b| {
        b.iter(|| ipaddr_to_u128(black_box("")))
    });

    c.bench_function("ipaddr_to_u128_ipv4", |b| {
        b.iter(|| ipaddr_to_u128(black_box("127.0.0.1")))
    });

    c.bench_function("ipaddr_to_u128_ipv6", |b| {
        b.iter(|| ipaddr_to_u128(black_box("2001:db8:102:304:506:708:90a:b0c")))
    });
}

fn bench_net_tcp_flags(c: &mut Criterion) {
    c.bench_function("tcp_flags_syn_ack", |b| {
        b.iter(|| tcp_flags(black_box(u16::to_be(0x12))))
    });

    c.bench_function("tcp_flags_none", |b| b.iter(|| tcp_flags(black_box(0))));

    c.bench_function("tcp_flags_all", |b| {
        b.iter(|| tcp_flags(black_box(u16::to_be(0x3f))))
    });
}

fn bench_time_display(c: &mut Criterion) {
    c.bench_function("time_as_nanosecond_zero", |b| {
        b.iter(|| TimeAsNanosecond(black_box(0)).to_string())
    });

    c.bench_function("time_as_nanosecond_seconds", |b| {
        b.iter(|| TimeAsNanosecond(black_box(1_000_000_000)).to_string())
    });

    c.bench_function("time_as_nanosecond_milliseconds", |b| {
        b.iter(|| TimeAsNanosecond(black_box(1_234_567_890)).to_string())
    });

    c.bench_function("time_as_nanosecond_hours", |b| {
        b.iter(|| TimeAsNanosecond(black_box(3_723_000_000_000)).to_string())
    });
}

fn bench_nanoseconds_display(c: &mut Criterion) {
    c.bench_function("nanoseconds_ns", |b| {
        b.iter(|| Nanoseconds(black_box(999)).to_string())
    });

    c.bench_function("nanoseconds_us", |b| {
        b.iter(|| Nanoseconds(black_box(1_000)).to_string())
    });

    c.bench_function("nanoseconds_ms", |b| {
        b.iter(|| Nanoseconds(black_box(1_000_000)).to_string())
    });

    c.bench_function("nanoseconds_s", |b| {
        b.iter(|| Nanoseconds(black_box(1_000_000_000)).to_string())
    });
}

criterion_group!(
    benches,
    bench_bytes_display,
    bench_cstring_from_bytes_lossy,
    bench_net_u32_to_ipaddr_v4,
    bench_net_u128_to_ipaddr_v6,
    bench_net_ipaddr_to_u128,
    bench_net_tcp_flags,
    bench_time_display,
    bench_nanoseconds_display
);
criterion_main!(benches);
