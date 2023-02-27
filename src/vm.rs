pub mod palloc;
pub mod ptable;

use palloc::*;
use ptable::kpage_init;
use crate::hw::param::*;

static mut PAGEPOOL: *mut PagePool = core::ptr::null_mut(); // *mut dyn Palloc

type VirtAddress = usize;
type PhysAddress = *mut usize;


#[derive(Debug)]
enum VmError {
    OutOfPages,
    PartialPalloc,
    PallocFail,
    PfreeFail,
}

trait Palloc {
    fn palloc(&mut self) -> Result<Page, VmError>;
    fn pfree(&mut self, size: usize) -> Result<(), VmError>;
}

pub fn init() {
    unsafe {
        PAGEPOOL = &mut PagePool::new(bss_end(), dram_end());
    }
    log!(Debug, "Successfully initialized kernel page pool...");

    // Map text, data, heap into kernel memory
    kpage_init();
}
