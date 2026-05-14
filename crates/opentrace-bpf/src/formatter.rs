// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.

use std::io::{self, Write};

use serde::Serialize;

use crate::symbols::SymbolResolver;

pub trait Formatter<T> {
    fn format<W:Write, R:SymbolResolver>(&self, w: &mut W, event: &T, resolver: &R) -> io::Result<()>;
}

// JsonFormatter<T>[#TODO] (shoule add some comments )
#[derive(Default)]
pub struct JsonFormatter;
impl<E: Serialize> Formatter<E> for JsonFormatter {
    fn format<W:Write, R:SymbolResolver>(&self, w: &mut W, event: &E, resolver: &R) -> io::Result<()>
    {
        serde_json::to_writer(w, event).map_err(io::Error::other)
    }
}
