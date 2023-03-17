//! Process handle and utilities.
// use alloc::boxed::Box;

// extern crate alloc;

use alloc::boxed::Box;

use crate::collection::BalBst;
use crate::hw::HartContext;
use crate::trap::TrapFrame;
use crate::vm::ptable::PageTable;
use crate::vm::Resource;

pub struct Process {
    id: usize,
    address_space: BalBst<Box<dyn Resource>>, // todo: Balanced BST of Resources
    state: ProcessState,
    pgtbl: PageTable,
    trapframe: TrapFrame,
    ctx_regs: HartContext,
}

pub enum ProcessState {
    Ready,
    Run,
    Wait,
    Sleep,
    Dead,
}
