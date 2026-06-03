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

#[cfg(test)]
mod tests {
    use super::{load_and_dispatch, load_and_dispath_with};
    use crate::exporter::Exporter;

    #[derive(Default)]
    struct VecExporter<T> {
        events: Vec<T>,
    }

    impl<T> Exporter<T> for VecExporter<T> {
        fn dispatch(&mut self, event: T) {
            self.events.push(event);
        }
    }

    #[test]
    fn load_and_dispatch_ignores_short_buffers() {
        let mut exporter = VecExporter::<u32>::default();

        load_and_dispatch::<u32, _>(&[1, 2], &mut exporter);

        assert!(exporter.events.is_empty());
    }

    #[test]
    fn load_and_dispatch_reads_plain_old_data() {
        let mut exporter = VecExporter::<u32>::default();
        let bytes = 42_u32.to_ne_bytes();

        load_and_dispatch::<u32, _>(&bytes, &mut exporter);

        assert_eq!(exporter.events, vec![42]);
    }

    #[test]
    fn load_and_dispatch_with_uses_handler_result() {
        let mut exporter = VecExporter::<u32>::default();

        load_and_dispath_with(&[1, 2, 3], &mut exporter, |data| Some(data.len() as u32));
        load_and_dispath_with(&[], &mut exporter, |_| None);

        assert_eq!(exporter.events, vec![3]);
    }
}
