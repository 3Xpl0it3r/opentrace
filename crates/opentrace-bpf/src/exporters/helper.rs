use super::Exporter;

// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.
//
pub(crate) fn load_and_dispatch<T, E>(data: &[u8], exporter: &mut E)
where
    E: Exporter<T>,
{
    if data.len() < std::mem::size_of::<T>() {
        return;
    }
    exporter.dispatch(unsafe { std::ptr::read(data.as_ptr() as *const T) });
}

pub(crate) fn load_and_dispath_with<T, E, F>(data: &[u8], exporter: &mut E, handle: F)
where
    E: Exporter<T>,
    F: FnOnce(&[u8]) -> Option<T>,
{
    if let Some(event) = handle(data) {
        exporter.dispatch(event);
    }
}
