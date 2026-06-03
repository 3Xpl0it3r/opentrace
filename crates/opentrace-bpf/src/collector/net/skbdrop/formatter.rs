use std::mem;

use crate::format::StreamFormatter;
use crate::symbolizer::{Source, SymbolizeInput, Symbolizer};
use crate::types::net::{AddrV4, AddrV6};

use super::Event;

// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.
pub struct DefaultFormatter<'a> {
    symbolizer: &'a dyn Symbolizer,
    source: Source<'a>,
}

impl<'a> DefaultFormatter<'a> {
    pub fn new(symbolizer: &'a dyn Symbolizer) -> Self {
        Self {
            symbolizer,
            source: Source::Kernel,
        }
    }
}

impl StreamFormatter<Event> for DefaultFormatter<'_> {
    fn format<W: std::io::Write>(&self, w: &mut W, event: &Event) -> Result<(), std::io::Error> {
        write_endpoints(w, event)?;
        writeln!(w, "reason: {}", event.drop_source_str())?;
        self.write_stack(w, event)?;
        writeln!(w, "{}", "---+---".repeat(10))
    }
}

/// 输出 "src:sport -> dst:dport" 一行，按 IP 版本走不同地址格式。
fn write_endpoints<W: std::io::Write>(w: &mut W, event: &Event) -> Result<(), std::io::Error> {
    let sport = u16::from_be(event.l4_info.sport);
    let dport = u16::from_be(event.l4_info.dport);
    match event.l3_info.ip_version {
        4 => writeln!(
            w,
            "{}:{} -> {}:{}",
            AddrV4::from(event.l3_info.saddr),
            sport,
            AddrV4::from(event.l3_info.daddr),
            dport,
        ),
        6 => writeln!(
            w,
            "{}:{} -> {}:{}",
            AddrV6::from(event.l3_info.saddr),
            sport,
            AddrV6::from(event.l3_info.daddr),
            dport,
        ),
        _ => writeln!(w, "0.0.0.0:{} -> 0.0.0.0:{}", sport, dport),
    }
}

impl DefaultFormatter<'_> {
    fn write_stack<W: std::io::Write>(
        &self,
        w: &mut W,
        event: &Event,
    ) -> Result<(), std::io::Error> {
        if event.stack_size <= 0 {
            return Ok(());
        }
        writeln!(w, "stack:")?;
        let stk_cnt = (event.stack_size as usize) / mem::size_of::<u64>();
        for addr in event.stack[..stk_cnt.min(event.stack.len())].iter() {
            let symb = self.symbolizer.resolve(SymbolizeInput {
                source: self.source.clone(),
                addr: *addr,
            });
            writeln!(w, "    {}", symb.name)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    /* use super::event::DROP_SRC_KFREE_SKB; */
    use super::*;
    use crate::symbolizer::ResolvedSymbol;
    use crate::types::net::{Addr, L2Info, L3Info, L4Info};
    use rstest::rstest;
    use std::borrow::Cow;
    const DROP_SRC_KFREE_SKB: u8 = 1;

    // Mock Symbolizer 实现
    struct MockSymbolizer;

    impl Symbolizer for MockSymbolizer {
        fn resolve(&self, input: SymbolizeInput) -> ResolvedSymbol<'_> {
            ResolvedSymbol {
                name: Cow::Owned(format!("func_{:x}", input.addr)),
                start_addr: input.addr,
                offset: 0,
            }
        }
    }

    fn make_event_v4(src: [u8; 4], dst: [u8; 4], sport: u16, dport: u16) -> Event {
        Event {
            l2_info: L2Info { eth_proto: 0 },
            l3_info: L3Info {
                saddr: Addr {
                    v4addr: u32::from_ne_bytes(src),
                },
                daddr: Addr {
                    v4addr: u32::from_ne_bytes(dst),
                },
                tot_len: 0,
                ip_version: 4,
                l4_proto: 6,
            },
            l4_info: L4Info {
                sport: u16::to_be(sport),
                dport: u16::to_be(dport),
                tcpflags: 0,
            },
            stack_size: 0,
            stack: [0; 16],
            drop_reason: 0,
            drop_source: DROP_SRC_KFREE_SKB,
        }
    }

    fn make_event_v6(sport: u16, dport: u16) -> Event {
        Event {
            l2_info: L2Info { eth_proto: 0 },
            l3_info: L3Info {
                saddr: Addr {
                    v6addr: crate::types::net::AddrV6 { upper: 0, lower: 1 },
                },
                daddr: Addr {
                    v6addr: crate::types::net::AddrV6 { upper: 0, lower: 2 },
                },
                tot_len: 0,
                ip_version: 6,
                l4_proto: 6,
            },
            l4_info: L4Info {
                sport: u16::to_be(sport),
                dport: u16::to_be(dport),
                tcpflags: 0,
            },
            stack_size: 0,
            stack: [0; 16],
            drop_reason: 0,
            drop_source: DROP_SRC_KFREE_SKB,
        }
    }

    // ==================== write_endpoints 测试 ====================

    #[test]
    fn write_endpoints_ipv4() {
        let event = make_event_v4([192, 168, 1, 1], [10, 0, 0, 1], 12345, 80);
        let mut buf = Vec::new();
        write_endpoints(&mut buf, &event).unwrap();
        let output = String::from_utf8(buf).unwrap();
        assert!(output.contains("192.168.1.1:12345"));
        assert!(output.contains("10.0.0.1:80"));
    }

    #[test]
    fn write_endpoints_ipv6() {
        let event = make_event_v6(8080, 443);
        let mut buf = Vec::new();
        write_endpoints(&mut buf, &event).unwrap();
        let output = String::from_utf8(buf).unwrap();
        assert!(output.contains("8080"));
        assert!(output.contains("443"));
    }

    #[rstest]
    #[case(0, "0.0.0.0:80 -> 0.0.0.0:443")]
    #[case(99, "0.0.0.0:80 -> 0.0.0.0:443")]
    fn write_endpoints_unknown_ip_version(#[case] version: u16, #[case] expected: &str) {
        let event = Event {
            l2_info: L2Info { eth_proto: 0 },
            l3_info: L3Info {
                saddr: Addr { v4addr: 0 },
                daddr: Addr { v4addr: 0 },
                tot_len: 0,
                ip_version: version,
                l4_proto: 6,
            },
            l4_info: L4Info {
                sport: u16::to_be(80),
                dport: u16::to_be(443),
                tcpflags: 0,
            },
            stack_size: 0,
            stack: [0; 16],
            drop_reason: 0,
            drop_source: DROP_SRC_KFREE_SKB,
        };
        let mut buf = Vec::new();
        write_endpoints(&mut buf, &event).unwrap();
        let output = String::from_utf8(buf).unwrap().trim().to_string();
        assert_eq!(output, expected);
    }

    // ==================== DefaultFormatter 测试 ====================

    #[test]
    fn default_formatter_outputs_reason() {
        let symbolizer = MockSymbolizer;
        let formatter = DefaultFormatter::new(&symbolizer);
        let event = make_event_v4([127, 0, 0, 1], [127, 0, 0, 2], 8080, 80);
        let mut buf = Vec::new();
        formatter.format(&mut buf, &event).unwrap();
        let output = String::from_utf8(buf).unwrap();
        assert!(output.contains("reason: kfree_skb"));
    }

    #[test]
    fn default_formatter_no_stack_when_size_zero() {
        let symbolizer = MockSymbolizer;
        let formatter = DefaultFormatter::new(&symbolizer);
        let event = make_event_v4([127, 0, 0, 1], [127, 0, 0, 2], 8080, 80);
        let mut buf = Vec::new();
        formatter.format(&mut buf, &event).unwrap();
        let output = String::from_utf8(buf).unwrap();
        assert!(!output.contains("stack:"));
    }

    #[test]
    fn default_formatter_with_stack() {
        let symbolizer = MockSymbolizer;
        let formatter = DefaultFormatter::new(&symbolizer);
        let mut event = make_event_v4([127, 0, 0, 1], [127, 0, 0, 2], 8080, 80);
        event.stack_size = 16; // 2 个 u64
        event.stack[0] = 0x1000;
        event.stack[1] = 0x2000;
        let mut buf = Vec::new();
        formatter.format(&mut buf, &event).unwrap();
        let output = String::from_utf8(buf).unwrap();
        assert!(output.contains("stack:"));
        assert!(output.contains("func_1000"));
        assert!(output.contains("func_2000"));
    }

    #[test]
    fn default_formatter_negative_stack_size_no_stack() {
        let symbolizer = MockSymbolizer;
        let formatter = DefaultFormatter::new(&symbolizer);
        let mut event = make_event_v4([127, 0, 0, 1], [127, 0, 0, 2], 8080, 80);
        event.stack_size = -1;
        let mut buf = Vec::new();
        formatter.format(&mut buf, &event).unwrap();
        let output = String::from_utf8(buf).unwrap();
        assert!(!output.contains("stack:"));
    }
}
