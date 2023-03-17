//! Virtual Memory
mod palloc;
pub mod process;
pub mod ptable;
pub mod vmalloc;
pub mod global;

use crate::hw::param::*;
use core::cell::OnceCell;
use core::alloc::{GlobalAlloc, Layout};
use alloc::boxed::Box;

use palloc::*;
use process::Process;
use ptable::kpage_init; //, PageTable};
use global::Galloc;

/// Global physical page pool allocated by the kernel physical allocator.
static mut PAGEPOOL: OnceCell<PagePool> = OnceCell::new();
#[global_allocator]
static mut GLOBAL: GlobalWrapper = GlobalWrapper { inner: OnceCell::new(), };

struct GlobalWrapper {
    inner: OnceCell<Galloc>,
}

unsafe impl GlobalAlloc for GlobalWrapper {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        self.inner.get().unwrap().alloc(layout)
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        self.inner.get().unwrap().dealloc(ptr, layout)
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        self.inner.get().unwrap().alloc_zeroed(layout)
    }

    unsafe fn realloc(
        &self,
        ptr: *mut u8,
        layout: Layout,
        new_size: usize
    ) -> *mut u8 {
        self.inner.get().unwrap().realloc(ptr, layout, new_size)
    }
}

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

/// Moving to `mod process`
pub trait Resource {}

/// Moving to `mod <TBD>`
pub struct TaskList {
    head: Option<Box<Process>>,
}

/// Moving to `mod <TBD>`
pub struct TaskNode {
    proc: Option<Box<Process>>,
    prev: Option<Box<TaskNode>>,
    next: Option<Box<TaskNode>>,
}

/// See `vm::vmalloc::Kalloc::alloc`.
// pub fn kalloc(size: usize) -> Result<*mut usize, vmalloc::KallocError> {
//     unsafe { VMALLOC.get_mut().unwrap().alloc(size) }
// }

/// See `vm::vmalloc::Kalloc::free`.
// pub fn kfree<T>(ptr: *mut T) {
//     unsafe { VMALLOC.get_mut().unwrap().free(ptr) }
// }

fn palloc() -> Result<Page, VmError> {
    unsafe { PAGEPOOL.get_mut().unwrap().palloc() }
}

fn pfree(page: Page) -> Result<(), VmError> {
    unsafe { PAGEPOOL.get_mut().unwrap().pfree(page) }
}

/// Initialize the kernel VM system.
/// First, setup the kernel physical page pool.
/// We start the pool at the end of the .bss section, and stop at the end of physical memory.
/// Next, we map physical memory into the kernel's physical memory 1:1.
/// Next, initialize the kernel virtual memory allocator pool.
/// Finally we set the global kernel page table `KPGTABLE` variable to point to the
/// kernel's page table struct.
pub fn init() -> Result<(), PagePool> {
    unsafe {
        match PAGEPOOL.set(PagePool::new(bss_end(), dram_end())) {
            Ok(_) => {}
            Err(_) => {
                panic!("vm double init.")
            }
        }
    }
    log!(Debug, "Successfully initialized kernel page pool...");

    unsafe {
        match GLOBAL.inner.set(Galloc::new(PAGEPOOL.get_mut().unwrap())) {
            Ok(_) => {},
            Err(_) => {
                panic!("vm double init.")
            }
        }
    }

    // Map text, data, stacks, heap into kernel page table.
    match kpage_init() {
        Ok(pt) => pt.write_satp(),
        Err(_) => {
            panic!();
        }
    }
    Ok(())
}

/// A test designed to be used with GDB.
/// Allocate A, then B. Free A, then B.
pub unsafe fn test_palloc() {
    let one = PAGEPOOL.get_mut().unwrap().palloc().unwrap();
    one.addr.write(0xdeadbeaf);

    let many  = PAGEPOOL.get_mut().unwrap().palloc_plural(5).unwrap();
    many.write_bytes(5, 512 * 2);

    let _ = PAGEPOOL.get_mut().unwrap().pfree(one);
    let _ = PAGEPOOL.get_mut().unwrap().pfree_plural(many, 5);
    
    log!(Debug, "Successful test of page allocation and freeing...");
}

pub unsafe fn test_galloc() {
    use alloc::collections;
    {
        // Simple test. It works!
        let mut one = Box::new(5);
        let a_one: *mut u32 = one.as_mut();
        assert_eq!(*one, *a_one);

        // Slightly more interesting... it also works! Look at GDB
        // and watch for the zone headers + chunk headers indicating 'in use' and
        // 'chunk size'. Then watch as these go out of scope.
        let mut one_vec: Box<collections::VecDeque<u32>> = Box::default();
        one_vec.push_back(555);
        one_vec.push_front(111);
        let _a_vec: *mut collections::VecDeque<u32> = one_vec.as_mut();
    }

    { // More than a page.
      let mut big: Box<[u64; 513]> = Box::new([0x8BADF00D;513]);
      let _a_big = big.as_mut();
    }

    log!(Debug, "Successful test of alloc crate...");
}

