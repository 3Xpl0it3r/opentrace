// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.

use std::marker::PhantomData;

use super::EventSink;
use crate::formatter::StreamFormatter;

// DefaultStdoutSink[#TODO] (should add some comments)
#[derive(Default)]
pub struct StreamWriterSink<E, F: StreamFormatter<E>, W: std::io::Write> {
    pub formater: F,
    _phantom: PhantomData<E>,
    pub writer: W,
}

impl<E, F: StreamFormatter<E>, W: std::io::Write> StreamWriterSink<E, F, W> {
    pub fn new(w: W, formatter: F) -> Self {
        Self {
            formater: formatter,
            writer: w,
            _phantom: PhantomData,
        }
    }
}

impl<E, F: StreamFormatter<E>, W: std::io::Write> EventSink<E> for StreamWriterSink<E, F, W> {
    fn dispatch(&mut self, event: E) {
        let _ = self.formater.format(&mut self.writer, &event);
    }
}

#[cfg(test)]
mod tests {
    use std::io;

    use super::StreamWriterSink;
    use crate::formatter::StreamFormatter;
    use crate::sink::EventSink;

    #[derive(Default)]
    struct TestFormatter;

    impl StreamFormatter<u32> for TestFormatter {
        fn format<W: io::Write>(&self, w: &mut W, value: &u32) -> io::Result<()> {
            write!(w, "value={value}")
        }
    }

    #[test]
    fn dispatch_writes_formatted_event_to_writer() {
        let mut sink = StreamWriterSink::new(Vec::new(), TestFormatter);

        sink.dispatch(42);

        assert_eq!(String::from_utf8(sink.writer).unwrap(), "value=42");
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
        let mut sink = StreamWriterSink::new(Vec::new(), FailingFormatter);

        sink.dispatch(42);

        assert!(sink.writer.is_empty());
    }

    #[test]
    fn multiple_dispatches_accumulate() {
        let mut sink = StreamWriterSink::new(Vec::new(), TestFormatter);

        sink.dispatch(1);
        sink.dispatch(2);
        sink.dispatch(3);

        assert_eq!(
            String::from_utf8(sink.writer).unwrap(),
            "value=1value=2value=3"
        );
    }

    #[test]
    fn dispatches_into_non_empty_writer() {
        let mut buf = Vec::from(b"prefix|".as_slice());
        let mut sink = StreamWriterSink::new(&mut buf, TestFormatter);

        sink.dispatch(99);

        let output = String::from_utf8(buf).unwrap();
        assert!(output.starts_with("prefix|"));
        assert!(output.contains("value=99"));
    }
}
