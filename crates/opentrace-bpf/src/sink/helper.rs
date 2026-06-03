// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.

use super::EventSink;
pub(crate) fn load_and_dispatch<T, S>(data: &[u8], sink: &mut S)
where
    S: EventSink<T>,
{
    if data.len() < std::mem::size_of::<T>() {
        return;
    }
    sink.dispatch(unsafe { std::ptr::read(data.as_ptr() as *const T) });
}

pub(crate) fn load_and_dispath_with<T, S, F>(data: &[u8], sink: &mut S, handle: F)
where
    S: EventSink<T>,
    F: FnOnce(&[u8]) -> Option<T>,
{
    if let Some(event) = handle(data) {
        sink.dispatch(event);
    }
}

#[cfg(test)]
mod tests {
    use super::{load_and_dispatch, load_and_dispath_with};
    use crate::sink::EventSink;

    #[derive(Default)]
    struct VecSink<T> {
        events: Vec<T>,
    }

    impl<T> EventSink<T> for VecSink<T> {
        fn dispatch(&mut self, event: T) {
            self.events.push(event);
        }
    }

    #[test]
    fn load_and_dispatch_ignores_short_buffers() {
        let mut sink = VecSink::<u32>::default();

        load_and_dispatch::<u32, _>(&[1, 2], &mut sink);

        assert!(sink.events.is_empty());
    }

    #[test]
    fn load_and_dispatch_reads_plain_old_data() {
        let mut sink = VecSink::<u32>::default();
        let bytes = 42_u32.to_ne_bytes();

        load_and_dispatch::<u32, _>(&bytes, &mut sink);

        assert_eq!(sink.events, vec![42]);
    }

    #[test]
    fn load_and_dispatch_with_uses_handler_result() {
        let mut sink = VecSink::<u32>::default();

        load_and_dispath_with(&[1, 2, 3], &mut sink, |data| Some(data.len() as u32));
        load_and_dispath_with(&[], &mut sink, |_| None);

        assert_eq!(sink.events, vec![3]);
    }
}
