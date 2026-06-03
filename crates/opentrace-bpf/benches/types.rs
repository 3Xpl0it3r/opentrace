use criterion::{Criterion, black_box, criterion_group, criterion_main};
use opentrace_bpf::types::net::{Addr, AddrV4, AddrV6, L2Info, L3Info, L4Info};

fn bench_addr_display(c: &mut Criterion) {
    let addr_v4 = AddrV4(u32::from_ne_bytes([192, 168, 1, 1]));
    let addr_v6 = AddrV6 {
        upper: u64::from_ne_bytes([0x20, 0x01, 0x0d, 0xb8, 0x01, 0x02, 0x03, 0x04]),
        lower: u64::from_ne_bytes([0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c]),
    };

    c.bench_function("addr_v4_display", |b| {
        b.iter(|| black_box(&addr_v4).to_string())
    });

    c.bench_function("addr_v6_display", |b| {
        b.iter(|| black_box(&addr_v6).to_string())
    });
}

fn bench_addr_conversion(c: &mut Criterion) {
    let addr = Addr {
        v4addr: u32::from_ne_bytes([10, 0, 0, 1]),
    };

    c.bench_function("addr_to_v4", |b| b.iter(|| AddrV4::from(black_box(addr))));

    c.bench_function("addr_to_v6", |b| b.iter(|| AddrV6::from(black_box(addr))));
}

fn bench_addr_from_array(c: &mut Criterion) {
    let arr = [
        u32::from_ne_bytes([0x20, 0x01, 0x0d, 0xb8]),
        u32::from_ne_bytes([0x01, 0x02, 0x03, 0x04]),
        u32::from_ne_bytes([0x05, 0x06, 0x07, 0x08]),
        u32::from_ne_bytes([0x09, 0x0a, 0x0b, 0x0c]),
    ];

    c.bench_function("addr_from_u32_array", |b| {
        b.iter(|| Addr::from(black_box(arr)))
    });
}

fn bench_l2_info_serialize(c: &mut Criterion) {
    let l2 = L2Info { eth_proto: 0x0800 };

    c.bench_function("l2_info_serialize", |b| {
        b.iter(|| serde_json::to_string(black_box(&l2)))
    });
}

fn bench_l3_info_serialize(c: &mut Criterion) {
    let l3 = L3Info {
        saddr: Addr {
            v4addr: u32::from_ne_bytes([10, 0, 0, 1]),
        },
        daddr: Addr {
            v4addr: u32::from_ne_bytes([10, 0, 0, 2]),
        },
        tot_len: 1500,
        ip_version: 4,
        l4_proto: 6,
    };

    c.bench_function("l3_info_serialize_ipv4", |b| {
        b.iter(|| serde_json::to_string(black_box(&l3)))
    });

    let l3_v6 = L3Info {
        saddr: Addr {
            v6addr: AddrV6 {
                upper: u64::from_ne_bytes([0x20, 0x01, 0x0d, 0xb8, 0x01, 0x02, 0x03, 0x04]),
                lower: u64::from_ne_bytes([0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c]),
            },
        },
        daddr: Addr {
            v6addr: AddrV6 {
                upper: u64::from_ne_bytes([0x20, 0x01, 0x0d, 0xb8, 0x09, 0x0a, 0x0b, 0x0c]),
                lower: u64::from_ne_bytes([0x05, 0x06, 0x07, 0x08, 0x01, 0x02, 0x03, 0x04]),
            },
        },
        tot_len: 1500,
        ip_version: 6,
        l4_proto: 6,
    };

    c.bench_function("l3_info_serialize_ipv6", |b| {
        b.iter(|| serde_json::to_string(black_box(&l3_v6)))
    });
}

fn bench_l4_info_serialize(c: &mut Criterion) {
    let l4 = L4Info {
        sport: u16::to_be(1234),
        dport: u16::to_be(80),
        tcpflags: u16::to_be(0x12), // SYN-ACK
    };

    c.bench_function("l4_info_serialize", |b| {
        b.iter(|| serde_json::to_string(black_box(&l4)))
    });
}

criterion_group!(
    benches,
    bench_addr_display,
    bench_addr_conversion,
    bench_addr_from_array,
    bench_l2_info_serialize,
    bench_l3_info_serialize,
    bench_l4_info_serialize
);
criterion_main!(benches);
