use std::mem;

// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.
//采样最大栈深度
const SAMPLE_STACK_DEPTH: usize = 6;

#[derive(Clone)]
#[repr(C)]
pub struct Event {
    /* process_info: ProcessInfo, */
    pub kstack: [u64; 16],
    pub ustack: [u64; 16],
    pub kstack_sz: i64,
    pub ustack_sz: i64,
    pub timestamp: u64,
    pub cpu_id: u32,
}

impl Event {
    fn stack_count(size: i64, max: usize) -> usize {
        if size <= 0 {
            return 0;
        }
        ((size as usize) / mem::size_of::<u64>()).min(max)
    }

    #[inline]
    pub fn stack_size(&self) -> (usize, usize) {
        (
            Self::stack_count(self.ustack_sz, SAMPLE_STACK_DEPTH),
            Self::stack_count(self.kstack_sz, SAMPLE_STACK_DEPTH),
        )
    }
}

pub type StackEvent = (Vec<u64>, Vec<u64>);

impl From<Event> for StackEvent {
    fn from(event: Event) -> Self {
        let (ustk_size, kstk_size) = event.stack_size();
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
        (ustack, kstack)
    }
}
