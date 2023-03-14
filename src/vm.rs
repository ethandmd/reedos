//! Virtual Memory
pub mod palloc;
pub mod ptable;
pub mod process;
pub mod galloc;
pub mod vmalloc;

use crate::hw::param::*;
use crate::mem::Kbox;
use palloc::*;
use ptable::kpage_init; //, PageTable};
use process::Process;
use core::cell::OnceCell;

/// Global physical page pool allocated by the kernel physical allocator.
static mut PAGEPOOL: OnceCell<PagePool> = OnceCell::new();
static mut VMALLOC: OnceCell<vmalloc::Kalloc> = OnceCell::new();

/// (Still growing) list of kernel VM system error cases.
#[derive(Debug)]
pub enum VmError {
    OutOfPages,
    PartialPalloc,
    PallocFail,
    PfreeFail,
    GNoSpace,
    Koom,
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

pub fn kalloc(size: usize) -> Result<*mut usize, vmalloc::KallocError> {
    unsafe { VMALLOC.get_mut().unwrap().alloc(size) }
}

pub fn kfree(ptr: *mut usize) {
    unsafe { VMALLOC.get_mut().unwrap().free(ptr) }
}

/// Initialize the kernel VM system.
/// First, setup the kernel physical page pool.
/// We start the pool at the end of the .bss section, and stop at the end of physical memory.
/// Next, we map physical memory into the kernel's physical memory 1:1.
/// Finally we set the global kernel page table `KPGTABLE` variable to point to the
/// kernel's page table struct.
pub fn init() -> Result<(), PagePool>{
    unsafe {
        match PAGEPOOL.set(PagePool::new(bss_end(), dram_end())) {
            Ok(_) => {},
            Err(_) => {
                panic!("vm double init.")
            }
        }
    }
    log!(Debug, "Successfully initialized kernel page pool...");

    // Map text, data, heap into kernel memory
    match kpage_init() {
        Ok(pt) => {
            pt.write_satp()
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
