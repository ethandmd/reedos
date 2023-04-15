//! Target-hardware parameters and utilities.
pub mod param;
pub mod riscv;
pub mod hartlocal;

use crate::device::clint;
use crate::trap;
use crate::process::Process;
use riscv::*;

/// Callee saved registers.
pub struct HartContext {
    regs: [usize; 32],
}

/// Representation of riscv hart.
pub struct Hart {
    id: usize,
    process: Process,
    ctx_regs: HartContext,
}

/// Set up and enable the core local interrupt controller on each hart.
/// We write the machine mode trap vector register (mtvec) with the address
/// of our `src/asm` trap handler function.
pub fn timerinit() {
    let interval = 10_000_000; // May want to speed this up in the future.
    clint::set_mtimecmp(interval);

    // Set the machine trap vector to hold fn ptr to timervec.
    let timervec_fn = trap::__mtrapvec;
    write_mtvec(timervec_fn as usize);

    // Enable machine mode interrupts with mstatus reg.
    let mut mstatus = read_mstatus();
    mstatus |= MSTATUS_MIE;
    write_mstatus(mstatus);

    // Enable machine-mode timer interrupts.
    let mie = read_mie() | MIE_MTIE;
    write_mie(mie);
}
