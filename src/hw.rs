pub mod riscv;
pub mod param;

use core::arch::asm;

use riscv::*;
use crate::device::clint::Clint;
use crate::trap;

// Sets up the core local interrupt controller on each hart.
// We set up CLINT per hart before we start bootstrapping so
// we can handle interrupts in supervisor mode (as opposed to
// machine mode).
pub fn timerinit() {
    let interval = 100_000;
    let clint = Clint::new();
    clint.bump_mtimecmp(interval);

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
        let hartid = riscv::read_tp();
        log!(Debug, " HART{}, timervec_fn: {}, mtvec reg: {}", hartid, timervec_fn as usize, read_mtvec());
        log!(Debug, " HART{}, mstatus: {}, mstatus reg: {}", hartid, mstatus, read_mstatus());
        log!(Debug, " HART{}, mie: {}, mie reg: {}", hartid, mie, read_mie());

    }
}

pub unsafe extern "C" fn timervec() {
    //pub fn timervec_fn() -> fn() {
    // xv6-riscv/kernel/kernelvec.S
    //
    // 1. Store function arguments (a0-7)
    // in first 3 slots in scratchpad
    //
    // 2. Schedule timer interrupt by
    // adding our interval to mtimecmp reg
    // who's addr is saved in scratchpad
    //
    // 3. Setup s/w interrupt with sip reg
    // (supervisor interrupt pending) for
    // after this function returns with mret.
    //
    // 4. Restore regs.
    asm!(
        r#"
        #.globl timervec
        #.align 4
    #timervec:
        csrrw a0, mscratch, a0
        sd a1, 0(a0)
        sd a2, 8(a0)
        sd a3, 16(a0)

        ld a1, 24(a0)
        ld a2, 32(a0)

        # Recall a1 has addr of mtimecmp
        ld a3, 0(a1)
        # Add interval arg to mtimecmp
        add a3, a3, a2
        # Store value in mtimecmp reg
        sd a3, 0(a1)

        # Call supervisor s/w interrupt to happen
        # after this returns so the kernel can handle.
        li a1, 2
        csrw sip, a1

        ld a3, 16(a0)
        ld a2, 8(a0)
        ld a1, 0(a0)
        csrrw a0, mscratch, a0

        mret
        "#
    );
}

