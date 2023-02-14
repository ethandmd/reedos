pub mod riscv;
pub mod param;

use riscv::*;
use crate::device::clint;
use crate::trap;

// Sets up the core local interrupt controller on each hart.
// We set up CLINT per hart before we start bootstrapping so
// we can handle interrupts in supervisor mode (as opposed to
// machine mode).
pub fn timerinit() {
    let interval = 10_000_000;
    //clint::bump_mtimecmp(interval);

    // Set the machine trap vector to hold fn ptr to timervec.
    let timervec_fn = trap::__mtrapvec;
    write_mtvec(timervec_fn as usize);

    // Enable machine mode interrupts with mstatus reg.
    let mstatus = read_mstatus() | MSTATUS_MIE;
    write_mstatus(mstatus);

    // Enable machine-mode timer interrupts.
    let mie = read_mie() | MIE_MTIE;
    write_mie(mie);

    #[cfg(debug_assertions)] {
        let hartid = riscv::read_mhartid();
        log!(Debug, " HART{}, timervec_fn: {:#02x}, mtvec reg: {:#02x}", hartid, timervec_fn as usize, read_mtvec());
        log!(Debug, " HART{}, mie: {:#02x}, mie reg: {:#02x}", hartid, mie, read_mie());

    }
}
