pub mod riscv;
pub mod param;

use riscv::*;
use crate::trap::{self, clint};

// Sets up the core local interrupt controller on each hart.
// We set up CLINT per hart before we start bootstrapping so
// we can handle interrupts in supervisor mode (as opposed to
// machine mode).
pub fn timerinit() {
    let clint = param::CLINT_BASE;
    let hartid = read_mhartid();
    let interval = 1000000; // <- # no. cycles ~ 1/10 sec in qemu.
    write_clint(hartid, clint, interval);

    let mut clint = clint::Clint::new(clint);
    clint.init(hartid as usize, interval);

    // Set the machine trap vector to hold fn ptr to timervec.
    let timervec_fn = trap::__TIMERVEC;
    write_mtvec(timervec_fn);

    // Enable machine mode interrupts with mstatus reg.
    let mstatus = read_mstatus() | MSTATUS_MIE;
    write_mstatus(mstatus);

    // Enable machine-mode timer interrupts.
    let mie = read_mie() | MIE_MTIE;
    write_mie(mie);

    #[cfg(debug_assertions)] {
        log!(Debug, " HART{}, timervec_fn: {}, mtvec reg: {}", hartid, timervec_fn as usize, read_mtvec());
        log!(Debug, " HART{}, mstatus: {}, mstatus reg: {}", hartid, mstatus, read_mstatus());
        log!(Debug, " HART{}, mie: {}, mie reg: {}", hartid, mie, read_mie());

    }
}
