//! Setup for s/w timer interrupts.
use core::arch::global_asm;
use crate::param;
use crate::riscv;

// Core Local Interrupt Timer driver and functions.
// Use memory mapped I/O on CLINT base address to program
// interrupts and set up memory for mscratch.
pub struct Clint {
    base: usize,
    scratchpad: [[usize; 5]; param::NHART],
}

impl Clint {
    // Register new clint and setup scratch memory.
    pub fn new(base: usize) -> Self {
        let scratchpad = [[0;5]; param::NHART];
        Clint {
            base,
            scratchpad,
        }
    }
    
    // Initialize clint with appropriate addresses and interrupt interval in cycles.
    pub fn init(&mut self, hartid: usize, interval: u64) {
        // scratchpad[0..2] : timervec uses this area to save register values.
        self.scratchpad[hartid][3] = self.base + 0x4000 + 8*hartid;
        self.scratchpad[hartid][4] = interval as usize;
        
        let scratchpad_addr = self.scratchpad.as_mut_ptr() as usize;
        riscv::write_mscratch(scratchpad_addr);
    }
}
// Best/idiomatic practice here? ultimately going to need
// the address of the function timervec to be stored in reg.
//
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
global_asm!(r#"
    .globl timervec
    .align 4
timervec:
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

    li a1, 2
    csrw sip, a1

    ld a3, 16(a0)
    ld a2, 8(a0)
    ld a1, 0(a0)
    csrrw a0, mscratch, a0

    mret
"#);

//#[no_mangle]
pub unsafe extern "C" fn timervec() {}
