pub mod param;
pub mod riscv;

use crate::device::clint;
use crate::trap;
use riscv::*;

// Sets up the core local interrupt controller on each hart.
// We set up CLINT per hart before we start bootstrapping so
// we can handle interrupts in supervisor mode (as opposed to
// machine mode).
pub fn timerinit() {
    let interval = 10_000_000;
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
