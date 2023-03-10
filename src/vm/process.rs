use crate::hw::HartContext;
use crate::trap::TrapFrame;
use crate::vm::ptable::PageTable;
use crate::vm::Resource;
use crate::collection::BalBst;
use crate::mem::Kbox;

pub struct Process {
    id: usize,
    address_space: BalBst<Kbox<dyn Resource>>, // todo: Balanced BST of Resources
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
