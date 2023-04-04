//! Process handle and utilities.
// use alloc::boxed::Box;

// extern crate alloc;

// use alloc::boxed::Box;
use alloc::collections::BTreeSet;


// use crate::hw::HartContext;
// use crate::trap::TrapFrame;
use crate::vm::ptable::PageTable;
use crate::vm::PhysPageExtent;

pub struct Process {
    id: usize,
    // address_space: BTreeSet<Box<dyn Resource>>, // todo: Balanced BST of Resources
    state: ProcessState,
    pub pgtbl: PageTable,
    physPages: BTreeSet<PhysPageExtent>,
    // trapframe: TrapFrame,
    // ctx_regs: HartContext,
}

pub enum ProcessState {
    Unstarted,
    Ready,
    Running,
    Wait,
    Sleep,
    Dead,
}
