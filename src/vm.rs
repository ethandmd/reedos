pub mod palloc;
pub mod ptable;

use palloc::Kpools;
use ptable::kpage_init;
use crate::hw::param::{__bss_end, __memory_end};

use core::ptr;

pub fn init() {
    // Setup page allocation pool for harts + global
    let bss_end: usize = unsafe { ptr::addr_of!(__bss_end) as usize};
    let mem_end: usize = unsafe { ptr::addr_of!(__memory_end) as usize};
    let mut pool = Kpools::new(bss_end, mem_end);
    log!(Debug, "Successfully initialized kernel page pool...");

    // Map text, data, heap into kernel memory
    kpage_init(&mut pool);
}
