use criterion::{Criterion, black_box, criterion_group, criterion_main};
use opentrace_bpf::protocol::appproto::HttpParser;
use opentrace_bpf::protocol::{ProtoParser, eth_proto, ip_proto};

fn bench_http1_request_parse(c: &mut Criterion) {
    let parser = HttpParser::default();
    let request = b"GET /hello HTTP/1.1\r\nHost: example.com\r\nUser-Agent: test\r\n\r\n";

    c.bench_function("http1_request_parse_verbose_false", |b| {
        b.iter(|| parser.parse(black_box(request), request.len(), false))
    });

    c.bench_function("http1_request_parse_verbose_true", |b| {
        b.iter(|| parser.parse(black_box(request), request.len(), true))
    });
}

fn bench_http1_response_parse(c: &mut Criterion) {
    let parser = HttpParser::default();
    let response =
        b"HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: 13\r\n\r\nHello, World!";

    c.bench_function("http1_response_parse_verbose_false", |b| {
        b.iter(|| parser.parse(black_box(response), response.len(), false))
    });

    c.bench_function("http1_response_parse_verbose_true", |b| {
        b.iter(|| parser.parse(black_box(response), response.len(), true))
    });
}

fn bench_http2_parse(c: &mut Criterion) {
    let parser = HttpParser::default();
    // HTTP/2 preface + settings frame
    let mut data = Vec::from(b"PRI * HTTP/2.0\r\n\r\nSM\r\n\r\n".as_slice());
    data.extend_from_slice(&[0, 0, 0, 4, 0, 0, 0, 0, 0]); // SETTINGS frame

    c.bench_function("http2_preface_settings_parse", |b| {
        b.iter(|| parser.parse(black_box(&data), data.len(), false))
    });
}

fn bench_http2_headers_parse(c: &mut Criterion) {
    let parser = HttpParser::default();
    // HEADERS frame with indexed headers (GET /)
    let data = [0, 0, 2, 1, 0, 0, 0, 0, 1, 0x82, 0x84];

    c.bench_function("http2_indexed_headers_parse", |b| {
        b.iter(|| parser.parse(black_box(&data), data.len(), false))
    });
}

fn bench_http_hash_id(c: &mut Criterion) {
    let parser = HttpParser::default();
    let request = b"GET /hello HTTP/1.1\r\nHost: example.com\r\n\r\n";

    c.bench_function("http1_hash_id", |b| {
        b.iter(|| parser.hash_id(black_box(request), request.len()))
    });
}

fn bench_ethernet_protocol_parse(c: &mut Criterion) {
    c.bench_function("eth_proto_parse_ip", |b| {
        b.iter(|| eth_proto::parse(black_box("ip")))
    });

    c.bench_function("eth_proto_parse_ipv6", |b| {
        b.iter(|| eth_proto::parse(black_box("ipv6")))
    });

    c.bench_function("eth_proto_to_str", |b| {
        b.iter(|| eth_proto::to_str(black_box(eth_proto::ETH_P_IP)))
    });
}

fn bench_ip_protocol_parse(c: &mut Criterion) {
    c.bench_function("ip_proto_parse_tcp", |b| {
        b.iter(|| ip_proto::parse(black_box("tcp")))
    });

    c.bench_function("ip_proto_parse_udp", |b| {
        b.iter(|| ip_proto::parse(black_box("udp")))
    });

    c.bench_function("ip_proto_to_str", |b| {
        b.iter(|| ip_proto::to_str(black_box(ip_proto::TCP)))
    });
}

criterion_group!(
    benches,
    bench_http1_request_parse,
    bench_http1_response_parse,
    bench_http2_parse,
    bench_http2_headers_parse,
    bench_http_hash_id,
    bench_ethernet_protocol_parse,
    bench_ip_protocol_parse
);
criterion_main!(benches);
