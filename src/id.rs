/// This module defineds a utility id generation helper for objects
/// with unpredictable lifetimes. It relies on alloc.

use alloc::collections::BTreeSet;

// TODO consider making this work like a guard, where when you drop an
// Id type (roughly transparent over a usize), then it auto-frees the
// id. Currently not implemented, as that requires each Id type to
// store a mut ptr (or a method of geting that) to the generator it is
// from. This is a lot of unsafe infrastructure for this, so I am not
// doing it now.

pub struct IdGenerator {
    counter: usize,
    in_use: BTreeSet<usize>,
}

impl IdGenerator {
    pub fn new() -> Self {
        Self {
            counter: 0,
            in_use: BTreeSet::new()
        }
    }

    pub fn generate(&mut self) -> usize {
        while self.in_use.contains(&self.counter) {
            self.counter = self.counter.wrapping_add(1);
        }

        self.in_use.insert(self.counter);
        let out = self.counter;
        self.counter = self.counter.wrapping_add(1); // slightly faster
        out
    }

    pub fn free(&mut self, id: usize) {
        if self.in_use.remove(&id) {}
        else {
            panic!("Double freed id {}!", id)
        }
    }
}
