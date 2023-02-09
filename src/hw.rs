pub mod riscv;
pub mod param;

use riscv::*;
use crate::trap::trapvec;

// Sets up the core local interrupt controller on each hart.
// We set up CLINT per hart before we start bootstrapping so
// we can handle interrupts in supervisor mode (as opposed to
// machine mode).
pub fn timerinit() {
    let clint = param::CLINT_BASE;
    let hartid = read_mhartid();
    let interval = 1000000; // <- # no. cycles ~ 1/10 sec in qemu.
    write_clint(hartid, clint, interval);

    let mut clint = trapvec::Clint::new(clint);
    clint.init(hartid as usize, interval);

    // Set the machine trap vector to hold fn ptr to timervec.
    let timervec_fn = trapvec::timervec as *const ();
    write_mtvec(timervec_fn);

    // Enable machine mode interrupts with mstatus reg.
    write_mstatus(read_mstatus() | MSTATUS_MIE);

    // Enable machine-mode timer interrupts.
    write_mie(read_mie() | MIE_MTIE);
}
