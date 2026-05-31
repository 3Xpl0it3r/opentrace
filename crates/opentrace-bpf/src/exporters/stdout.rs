// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.

use std::marker::PhantomData;

use super::Exporter;
use crate::formatter::StreamFormatter;

// DefaultStdoutExporter[#TODO] (shoule add some comments )
#[derive(Default)]
pub struct DefaultStdoutExporter<E, F: StreamFormatter<E>> {
    formater: F,
    _phantom: PhantomData<E>,
}

impl<E, F: StreamFormatter<E>> DefaultStdoutExporter<E, F> {
    pub fn new(formatter: F) -> Self {
        Self {
            formater: formatter,
            _phantom: PhantomData,
        }
    }
}

impl<E, F: StreamFormatter<E>> Exporter<E> for DefaultStdoutExporter<E, F> {
    fn dispatch(&mut self, event: E) {
        let mut stdout = std::io::stdout().lock();
        let _ = self.formater.format(&mut stdout, &event);
    }
}
