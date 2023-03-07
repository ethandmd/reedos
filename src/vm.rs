//! Virtual Memory
pub mod palloc;
pub mod ptable;
pub mod process;
pub mod galloc;

use crate::hw::param::*;
use crate::mem::Kbox;
use palloc::*;
use galloc::GAlloc;
use ptable::{kpage_init, PageTable};
use process::Process;
use core::cell::OnceCell;

/// Global physical page pool allocated by the kernel physical allocator.
//static mut PAGEPOOL: PagePool = PagePool::new(bss_end(), dram_end());
static mut PAGEPOOL: OnceCell<PagePool> = OnceCell::new();
/// Global kernel page table.
pub static mut KPGTABLE: *mut PageTable = core::ptr::null_mut();

/// (Still growing) list of kernel VM system error cases.
#[derive(Debug)]
pub enum VmError {
    OutOfPages,
    PartialPalloc,
    PallocFail,
    PfreeFail,
    GNoSpace,
}

pub trait Resource {}

pub struct TaskList {
    head: Option<Kbox<Process>>, // TODO:Convert to Option<Kbox<Process>>>
}

pub struct TaskNode {
    proc: Option<Kbox<Process>>,
    prev: Option<Kbox<TaskNode>>,
    next: Option<Kbox<TaskNode>>,
}

/// Initialize the kernel VM system.
/// First, setup the kernel physical page pool.
/// We start the pool at the end of the .bss section, and stop at the end of physical memory.
/// Next, we map physical memory into the kernel's physical memory 1:1.
/// Finally we set the global kernel page table `KPGTABLE` variable to point to the
/// kernel's page table struct.
pub fn init() -> Result<(), PagePool>{
    unsafe { PAGEPOOL.set(PagePool::new(bss_end(), dram_end()))?; }
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
    Ok(())
}

pub unsafe fn test_palloc() {
    let allocd = PAGEPOOL.get_mut().unwrap().palloc().unwrap();
    //println!("allocd addr: {:?}", allocd.addr);
    allocd.addr.write(0xdeadbeaf);
    let _ = PAGEPOOL.get_mut().unwrap().pfree(allocd);
    log!(Debug, "Successful test of page allocation and freeing...");
}
