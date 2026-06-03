// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.

use std::marker::PhantomData;

use super::Exporter;
use crate::formatter::StreamFormatter;

// DefaultStdoutExporter[#TODO] (shoule add some comments )
#[derive(Default)]
pub struct StreamWriterExpoter<E, F: StreamFormatter<E>, W: std::io::Write> {
    pub formater: F,
    _phantom: PhantomData<E>,
    pub writer: W,
}

impl<E, F: StreamFormatter<E>, W: std::io::Write> StreamWriterExpoter<E, F, W> {
    pub fn new(w: W, formatter: F) -> Self {
        Self {
            formater: formatter,
            writer: w,
            _phantom: PhantomData,
        }
    }
}

impl<E, F: StreamFormatter<E>, W: std::io::Write> Exporter<E> for StreamWriterExpoter<E, F, W> {
    fn dispatch(&mut self, event: E) {
        let _ = self.formater.format(&mut self.writer, &event);
    }
}

#[cfg(test)]
mod tests {
    use std::io;

    use super::StreamWriterExpoter;
    use crate::exporter::Exporter;
    use crate::formatter::StreamFormatter;

    #[derive(Default)]
    struct TestFormatter;

    impl StreamFormatter<u32> for TestFormatter {
        fn format<W: io::Write>(&self, w: &mut W, value: &u32) -> io::Result<()> {
            write!(w, "value={value}")
        }
    }

    #[test]
    fn dispatch_writes_formatted_event_to_writer() {
        let mut exporter = StreamWriterExpoter::new(Vec::new(), TestFormatter);

        exporter.dispatch(42);

        assert_eq!(String::from_utf8(exporter.writer).unwrap(), "value=42");
    }

    #[derive(Default)]
    struct FailingFormatter;

    impl StreamFormatter<u32> for FailingFormatter {
        fn format<W: io::Write>(&self, _w: &mut W, _value: &u32) -> io::Result<()> {
            Err(io::Error::other("format failed"))
        }
    }

    #[test]
    fn dispatch_ignores_formatter_errors() {
        let mut exporter = StreamWriterExpoter::new(Vec::new(), FailingFormatter);

        exporter.dispatch(42);

        assert!(exporter.writer.is_empty());
    }

    #[test]
    fn multiple_dispatches_accumulate() {
        let mut exporter = StreamWriterExpoter::new(Vec::new(), TestFormatter);

        exporter.dispatch(1);
        exporter.dispatch(2);
        exporter.dispatch(3);

        assert_eq!(
            String::from_utf8(exporter.writer).unwrap(),
            "value=1value=2value=3"
        );
    }

    #[test]
    fn dispatches_into_non_empty_writer() {
        let mut buf = Vec::from(b"prefix|".as_slice());
        let mut exporter = StreamWriterExpoter::new(&mut buf, TestFormatter);

        exporter.dispatch(99);

        let output = String::from_utf8(buf).unwrap();
        assert!(output.starts_with("prefix|"));
        assert!(output.contains("value=99"));
    }
}
