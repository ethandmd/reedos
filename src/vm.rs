pub mod palloc;
pub mod ptable;

use palloc::Kpools;
use ptable::kpage_init;
use crate::hw::param::{BSS_END, DRAM_END};

pub fn init() {
    // Setup page allocation pool for harts + global
    let bss_end = unsafe { BSS_END };
    let mem_end = unsafe { DRAM_END };
    let mut pool = Kpools::new(bss_end, mem_end);

    // Map text, data, heap into kernel memory
    let kpage_root = kpage_init(&mut pool);
}
