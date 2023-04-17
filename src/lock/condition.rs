//! Condition variable implementation
//!
//! Spins and does not block, used outside and around scheduling

use core::sync::atomic::{Ordering, AtomicUsize};
use core::hint::spin_loop;

pub struct ConditionVar {
    contents: AtomicUsize,
}

impl ConditionVar {
    pub fn new(val: usize) -> Self {
        Self {
            contents: AtomicUsize::new(val)
        }
    }

    pub fn spin_wait(&self, expected: usize) {
        while self.contents.load(Ordering::Acquire) != expected {
            spin_loop();
        }
    }

    pub fn update(&mut self, new_val: usize) {
        self.contents.store(new_val, Ordering::Release);
    }
}

