//! Setup for s/w timer interrupts.
use crate::hw::param;
use crate::hw::riscv;

// Core Local Interruptor driver and functions.
// Use memory mapped I/O on CLINT base address to program
// interrupts and set up memory for mscratch.
pub struct Clint {
    base: usize,
    //scratchpad: [[usize; 5]; param::NHART],
}

impl Clint {
    // Register new clint and setup scratch memory.
    pub fn new() -> Self {
        //let scratchpad = [[0; 5]; param::NHART];
        Clint { base: param::CLINT_BASE }//, scratchpad }
    }
    
    // mtimecmp reg is at base + 0x4000
    // mtime reg is base + 0xbff8
    pub fn bump_mtimecmp(&self, interval: u64) {
        let hartid = riscv::read_tp() as usize;
        let base = self.base as *mut usize;
        unsafe {
            // One mtime register for all cores.
            let mtime = base.byte_add(0xBFF8).read_volatile();
            // mtimecmp register per core.
            base.byte_add(0x4000 + 8*hartid).write_volatile(mtime + interval as usize);
        }
    }
}
