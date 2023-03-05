pub mod palloc;
pub mod ptable;

use crate::hw::param::*;
use palloc::*;
use ptable::{kpage_init, PageTable};

static mut PAGEPOOL: *mut PagePool = core::ptr::null_mut(); // *mut dyn Palloc
pub static mut KPGTABLE: *mut PageTable = core::ptr::null_mut();

type VirtAddress = usize;
type PhysAddress = *mut usize;

#[derive(Debug)]
pub enum VmError {
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
    match kpage_init() {
        Ok(mut pt) => unsafe {
            KPGTABLE = &mut pt;
        },
        Err(_) => {
            panic!();
        }
    }
}
