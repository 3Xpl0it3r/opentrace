use std::mem;

// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.
//采样最大栈深度
const SAMPLE_STACK_DEPTH: usize = 6;

#[derive(Clone)]
#[repr(C)]
pub(super) struct InnerEvent {
    /* process_info: ProcessInfo, */
    pub kstack: [u64; 16],
    pub ustack: [u64; 16],
    pub kstack_sz: i64,
    pub ustack_sz: i64,
    pub timestamp: u64,
    pub cpu_id: u32,
}

impl InnerEvent {
    #[inline]
    fn stack_count(size: i64, max: usize) -> usize {
        if size <= 0 {
            return 0;
        }
        ((size as usize) / mem::size_of::<u64>()).min(max)
    }
}

pub struct Event {
    pub ustack: Vec<u64>,
    pub kstack: Vec<u64>,
}

impl From<InnerEvent> for Event {
    fn from(event: InnerEvent) -> Self {
        let ustk_size = InnerEvent::stack_count(event.ustack_sz, SAMPLE_STACK_DEPTH);
        let kstk_size = InnerEvent::stack_count(event.kstack_sz, SAMPLE_STACK_DEPTH);
        let ustack = if ustk_size != 0 {
            let mut buffer = Vec::with_capacity(ustk_size);
            buffer.extend(&mut event.ustack[..ustk_size].iter().rev());
            buffer
        } else {
            vec![]
        };
        let kstack = if kstk_size != 0 {
            let mut buffer = Vec::with_capacity(kstk_size);
            buffer.extend(&mut event.kstack[..kstk_size].iter().rev());
            buffer
        } else {
            vec![]
        };
        Self { ustack, kstack }
    }
}

#[cfg(test)]
mod tests {
    use super::{Event, InnerEvent};

    #[test]
    fn converts_stack_sizes_from_bytes_to_frame_counts_and_reverses() {
        let mut inner = InnerEvent {
            kstack: [0; 16],
            ustack: [0; 16],
            kstack_sz: 24,
            ustack_sz: 24,
            timestamp: 0,
            cpu_id: 0,
        };
        inner.ustack[..3].copy_from_slice(&[1, 2, 3]);
        inner.kstack[..3].copy_from_slice(&[10, 20, 30]);

        let event = Event::from(inner);

        assert_eq!(event.ustack, vec![3, 2, 1]);
        assert_eq!(event.kstack, vec![30, 20, 10]);
    }

    #[test]
    fn clamps_stack_depth_to_sample_limit() {
        let mut inner = InnerEvent {
            kstack: [0; 16],
            ustack: [0; 16],
            kstack_sz: 16 * 8,
            ustack_sz: 16 * 8,
            timestamp: 0,
            cpu_id: 0,
        };
        for (idx, frame) in inner.ustack.iter_mut().enumerate() {
            *frame = idx as u64;
        }
        for (idx, frame) in inner.kstack.iter_mut().enumerate() {
            *frame = idx as u64 + 100;
        }

        let event = Event::from(inner);

        assert_eq!(event.ustack.len(), super::SAMPLE_STACK_DEPTH);
        assert_eq!(event.kstack.len(), super::SAMPLE_STACK_DEPTH);
    }

    #[test]
    fn kstack_and_ustack_use_independent_sizes() {
        let mut inner = InnerEvent {
            kstack: [0; 16],
            ustack: [0; 16],
            kstack_sz: 16, // 2 frames
            ustack_sz: 32, // 4 frames (32/8=4)
            timestamp: 0,
            cpu_id: 0,
        };
        inner.kstack[..2].copy_from_slice(&[100, 101]);
        inner.ustack[..4].copy_from_slice(&[1, 2, 3, 4]);

        let event = Event::from(inner);

        assert_eq!(event.kstack, vec![101, 100]);
        assert_eq!(event.ustack, vec![4, 3, 2, 1]);
    }

    #[test]
    fn zero_stack_size_produces_empty_stack() {
        let event = Event::from(InnerEvent {
            kstack: [1; 16],
            ustack: [1; 16],
            kstack_sz: 0,
            ustack_sz: 0,
            timestamp: 0,
            cpu_id: 0,
        });

        assert!(event.ustack.is_empty());
        assert!(event.kstack.is_empty());
    }

    #[test]
    fn negative_stack_size_produces_empty_stack() {
        let event = Event::from(InnerEvent {
            kstack: [1; 16],
            ustack: [1; 16],
            kstack_sz: -1,
            ustack_sz: -1,
            timestamp: 0,
            cpu_id: 0,
        });

        assert!(event.ustack.is_empty());
        assert!(event.kstack.is_empty());
    }
}
