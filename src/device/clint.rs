//! Setup for s/w timer interrupts.
use crate::hw::param;
use crate::hw::riscv;

// mtimecmp reg is at base + 0x4000
// mtime reg is base + 0xbff8
pub fn bump_mtimecmp(interval: u64) {
    let hartid = riscv::read_mhartid() as usize;
    let base = param::CLINT_BASE as *mut usize;
    unsafe {
        // One mtime register for all cores.
        let mtime = base.byte_add(0xBFF8).read_volatile();
        // mtimecmp register per core.
        base.byte_add(0x4000 + 8*hartid).write_volatile(mtime + interval as usize);
    }
}

