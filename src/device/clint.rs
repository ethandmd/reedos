//! Setup for s/w timer interrupts.
use crate::hw::param;
use crate::hw::riscv;

pub fn read_mtime() -> u64 {
    let base = param::CLINT_BASE as *mut u64;
    let mtime: u64;
    unsafe {
        mtime = base.byte_add(0xBFF8).read_volatile();
    }
    mtime
}

<<<<<<< HEAD
// mtimecmp reg is at base + 0x4000
// mtime reg is base + 0xbff8
pub fn set_mtimecmp(interval: u64) {
    let hartid = riscv::read_mhartid() as usize;
    let base = param::CLINT_BASE as *mut usize;
    unsafe {
        // One mtime register for all cores.
        let mtime = base.byte_add(0xBFF8).read_volatile();
        // mtimecmp register per core.
        base.byte_add(0x4000 + 8*hartid).write_volatile(mtime + interval as usize);
=======
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
>>>>>>> 6e119cd (rebase 7 from main)
    }
}
