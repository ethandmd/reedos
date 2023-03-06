//! Virtual Memory
pub mod palloc;
pub mod ptable;
pub mod process;

use crate::hw::param::*;
use palloc::*;
use ptable::{kpage_init, PageTable};
use process::Process;

/// Global physical page pool allocated by the kernel physical allocator.
static mut PAGEPOOL: *mut PagePool = core::ptr::null_mut(); // *mut dyn Palloc
/// Global kernel page table.
pub static mut KPGTABLE: *mut PageTable = core::ptr::null_mut();

/// (Still growing) list of kernel VM system error cases.
#[derive(Debug)]
pub enum VmError {
    OutOfPages,
    PartialPalloc,
    PallocFail,
    PfreeFail,
}

/// Generic interface the phyiscal page allocator implements.
trait Palloc {
    fn palloc(&mut self) -> Result<Page, VmError>;
    fn pfree(&mut self, size: usize) -> Result<(), VmError>;
}

//pub trait Resource {}
pub struct Resource;

pub struct TaskList {
    head: Option<Process>, // TODO:Convert to Option<Kbox<Process>>>
}

/// Initialize the kernel VM system.
/// First, setup the kernel physical page pool.
/// We start the pool at the end of the .bss section, and stop at the end of physical memory.
/// Next, we map physical memory into the kernel's physical memory 1:1.
/// Finally we set the global kernel page table `KPGTABLE` variable to point to the
/// kernel's page table struct.
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
