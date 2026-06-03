// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.

//! 集成测试：protocols 模块
//!
//! 测试公开的协议解析 API

use opentrace_bpf::protocols::{
    MessageType, ParsedFrame, ProtoParser,
    app_protos::{HttpMethod, HttpParser},
    eth_proto, ip_proto,
};
use rstest::rstest;

// ==================== eth_proto 测试 ====================

#[rstest]
#[case("ip", eth_proto::ETH_P_IP)]
#[case("ipv4", eth_proto::ETH_P_IP)]
#[case("IP", eth_proto::ETH_P_IP)]
#[case("IPv4", eth_proto::ETH_P_IP)]
#[case("ipv6", eth_proto::ETH_P_IPV6)]
#[case("IPV6", eth_proto::ETH_P_IPV6)]
fn eth_proto_parse_supported(#[case] input: &str, #[case] expected: u16) {
    assert_eq!(eth_proto::parse(input).unwrap(), expected);
}

#[rstest]
#[case("arp")]
#[case("unknown")]
#[case("")]
fn eth_proto_parse_unsupported(#[case] input: &str) {
    assert!(eth_proto::parse(input).is_err());
}

#[rstest]
#[case(eth_proto::ETH_P_IP, "ETH_P_IP")]
#[case(eth_proto::ETH_P_IPV6, "ETH_P_IPV6")]
#[case(eth_proto::ETH_P_ARP, "ETH_P_ARP")]
#[case(0, "Unknown")]
fn eth_proto_to_str(#[case] proto: u16, #[case] expected: &str) {
    assert_eq!(eth_proto::to_str(proto), expected);
}

// ==================== ip_proto 测试 ====================

#[rstest]
#[case("tcp", ip_proto::TCP)]
#[case("udp", ip_proto::UDP)]
#[case("icmp", ip_proto::ICMP)]
#[case("TCP", ip_proto::TCP)]
#[case("Udp", ip_proto::UDP)]
fn ip_proto_parse_supported(#[case] input: &str, #[case] expected: u16) {
    assert_eq!(ip_proto::parse(input).unwrap(), expected);
}

#[rstest]
#[case("sctp")]
#[case("unknown")]
#[case("")]
fn ip_proto_parse_unsupported(#[case] input: &str) {
    assert!(ip_proto::parse(input).is_err());
}

#[rstest]
#[case(ip_proto::TCP, "IPPROTO_TCP")]
#[case(ip_proto::UDP, "IPPROTO_UDP")]
#[case(ip_proto::ICMP, "IPPROTO_ICMP")]
#[case(999, "UNKNOWN")]
fn ip_proto_to_str(#[case] proto: u16, #[case] expected: &str) {
    assert_eq!(ip_proto::to_str(proto), expected);
}

// ==================== HttpParser 测试 ====================

#[test]
fn http_parser_default() {
    let _parser = HttpParser::default();
}

#[test]
fn http_parser_parses_get_request() {
    let parser = HttpParser::default();
    let data = b"GET /hello HTTP/1.1\r\nHost: example.com\r\n\r\n";
    let mut frame = parser.parse(data, data.len(), false).unwrap();

    assert_eq!(frame.message_type(), MessageType::Request);
    assert_eq!(frame.target().as_deref(), Some("GET /hello"));
    assert!(frame.payload().is_none());
}

#[test]
fn http_parser_parses_post_request_with_body() {
    let parser = HttpParser::default();
    let data = b"POST /submit HTTP/1.1\r\nHost: example.com\r\nContent-Length: 5\r\n\r\nhello";
    let mut frame = parser.parse(data, data.len(), true).unwrap();

    assert_eq!(frame.message_type(), MessageType::Request);
    assert_eq!(frame.target().as_deref(), Some("POST /submit"));
    assert_eq!(frame.payload(), Some(Box::from("hello")));
}

#[test]
fn http_parser_parses_response() {
    let parser = HttpParser::default();
    let data = b"HTTP/1.1 200 OK\r\nContent-Length: 5\r\n\r\nhello";
    let mut frame = parser.parse(data, data.len(), true).unwrap();

    assert_eq!(frame.message_type(), MessageType::Response);
    assert_eq!(frame.payload(), Some(Box::from("hello")));
}

#[test]
fn http_parser_returns_none_for_invalid_data() {
    let parser = HttpParser::default();
    assert!(parser.parse(b"not http", 8, false).is_none());
}

#[test]
fn http_parser_hash_id_stable() {
    let parser = HttpParser::default();
    let data = b"GET /hello HTTP/1.1\r\n\r\n";
    let hash1 = parser.hash_id(data, data.len());
    let hash2 = parser.hash_id(data, data.len());
    assert_eq!(hash1, hash2);
    assert_ne!(hash1, 0);
}

#[test]
fn http_parser_hash_id_different_for_different_urls() {
    let parser = HttpParser::default();
    let hash1 = parser.hash_id(b"GET /a HTTP/1.1\r\n\r\n", 20);
    let hash2 = parser.hash_id(b"GET /b HTTP/1.1\r\n\r\n", 20);
    assert_ne!(hash1, hash2);
}

// ==================== HttpMethod 测试 ====================

#[rstest]
#[case(HttpMethod::Get, "GET", b"GET")]
#[case(HttpMethod::Post, "POST", b"POST")]
#[case(HttpMethod::Put, "PUT", b"PUT")]
#[case(HttpMethod::Delete, "DELETE", b"DELETE")]
#[case(HttpMethod::Patch, "PATCH", b"PATCH")]
#[case(HttpMethod::Head, "HEAD", b"HEAD")]
#[case(HttpMethod::Options, "OPTIONS", b"OPTIONS")]
#[case(HttpMethod::Connect, "CONNECT", b"CONNECT")]
#[case(HttpMethod::Trace, "TRACE", b"TRACE")]
fn http_method_properties(
    #[case] method: HttpMethod,
    #[case] expected_str: &str,
    #[case] expected_bytes: &[u8],
) {
    assert_eq!(method.as_str(), expected_str);
    assert_eq!(method.as_bytes(), expected_bytes);
    assert_eq!(format!("{}", method), expected_str);
}

// ==================== MessageType 测试 ====================

#[test]
fn message_type_equality() {
    assert_eq!(MessageType::Request, MessageType::Request);
    assert_eq!(MessageType::Response, MessageType::Response);
    assert_eq!(MessageType::Unknown, MessageType::Unknown);
    assert_ne!(MessageType::Request, MessageType::Response);
}
