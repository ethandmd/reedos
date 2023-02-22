pub mod riscv;
pub mod param;

use riscv::*;
<<<<<<< HEAD
use crate::device::clint;
use crate::trap;
=======
use crate::device::clint::Clint;
>>>>>>> 6e119cd (rebase 7 from main)

// Sets up the core local interrupt controller on each hart.
// We set up CLINT per hart before we start bootstrapping so
// we can handle interrupts in supervisor mode (as opposed to
// machine mode).
pub fn timerinit() {
    let interval = 10_000_000;
    clint::set_mtimecmp(interval);
    
    // Set the machine trap vector to hold fn ptr to timervec.
<<<<<<< HEAD
    let timervec_fn = trap::__mtrapvec;
    write_mtvec(timervec_fn as usize);
=======
    let timervec_fn = timervec;
    write_mtvec(timervec_fn);
>>>>>>> 6e119cd (rebase 7 from main)

    // Enable machine mode interrupts with mstatus reg.
    let mut mstatus = read_mstatus(); 
    mstatus |= MSTATUS_MIE;
    write_mstatus(mstatus);

    // Enable machine-mode timer interrupts.
    let mie = read_mie() | MIE_MTIE;
    write_mie(mie);    
}
