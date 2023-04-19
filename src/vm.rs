//! Virtual Memory
pub mod global;
mod palloc;
pub mod ptable;
pub mod vmalloc;

use crate::lock::mutex::Mutex;
use crate::hw::param::*;
use alloc::boxed::Box;
use core::alloc::{GlobalAlloc, Layout};
use core::cell::OnceCell;
use core::arch::asm;

use global::Galloc;
use palloc::*;
use ptable::{PageTable, kpage_init};

// For saftey reasons, no part of the page allocation process all the
// way up past this module can use rust dynamic allocation (global
// usage). This causes a dependency cycle in some places, and a in
// every case opens the possibility of deadlock between the global
// lock and the palloc lock. This is mostly relevant for
// request_phys_page

/// Global physical page pool allocated by the kernel physical allocator.
static mut PAGEPOOL: OnceCell<PagePool> = OnceCell::new();
#[global_allocator]
static mut GLOBAL: GlobalWrapper = GlobalWrapper {
    inner: OnceCell::new(),
};

struct GlobalWrapper {
    inner: OnceCell<Mutex<Galloc>>,
}

unsafe impl GlobalAlloc for GlobalWrapper {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        self.inner.get().unwrap().lock().alloc(layout)
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        self.inner.get().unwrap().lock().dealloc(ptr, layout)
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        self.inner.get().unwrap().lock().alloc_zeroed(layout)
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        self.inner.get().unwrap().lock().realloc(ptr, layout, new_size)
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


/// Initialize the kernel VM system.
/// First, setup the kernel physical page pool.
/// We start the pool at the end of the .bss section, and stop at the end of physical memory.
/// Next, we map physical memory into the kernel's physical memory 1:1.
/// Next, initialize the kernel virtual memory allocator pool.
///
/// TODO better error type
pub fn global_init() -> Result<PageTable, ()> {
    unsafe {
        match PAGEPOOL.set(PagePool::new(bss_end(), memory_end())) {
            Ok(_) => {}
            Err(_) => {
                panic!("vm double init.")
            }
        }
    }
    log!(Debug, "Successfully initialized kernel page pool...");

    unsafe {
        match GLOBAL.inner.set(Mutex::new(Galloc::new(PAGEPOOL.get_mut().unwrap()))) {
            Ok(_) => {}
            Err(_) => {
                panic!("vm double init.")
            }
        }
    }

    // Map text, data, stacks, heap into kernel page table.
    match kpage_init() {
        Ok(pt) => {
            return Ok(pt);
        },
        Err(_) => {
            panic!();
        }
    }
}

pub fn local_init(pt: &PageTable) {
    pt.write_satp();
    pagetable_interrupt_stack_setup(pt);
}

// TODO error type?
fn pagetable_interrupt_stack_setup(pt: &ptable::PageTable) {
    log!(Debug, "Writing kernel page table {:02X?}", pt.base);
    unsafe {
        asm!(
            "csrrw sp, sscratch, sp",
            "addi sp, sp, -8",
            "sd {page_table}, (sp)",
            "csrrw sp, sscratch, sp",
            page_table = in(reg) pt.base as usize
        );
    }
}

/// A test designed to be used with GDB.
/// Allocate A, then B. Free A, then B.
pub unsafe fn test_palloc() {
    let one = PAGEPOOL.get_mut().unwrap().palloc().unwrap();
    one.addr.write(0xdeadbeaf);

    let many = PAGEPOOL.get_mut().unwrap().palloc_plural(5).unwrap();
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

    log!(Debug, "Successful test of alloc crate...");
}

// -------------------------------------------------------------------


// /// See `vm::vmalloc::Kalloc::alloc`.
// pub fn kalloc(size: usize) -> Result<*mut usize, vmalloc::KallocError> {
//     unsafe { VMALLOC.get_mut().unwrap().alloc(size) }
// }

// /// See `vm::vmalloc::Kalloc::free`.
// pub fn kfree<T>(ptr: *mut T) {
//     unsafe { VMALLOC.get_mut().unwrap().free(ptr) }
// }

// for internal vm use only.
fn palloc() -> Result<Page, VmError> {
    unsafe { PAGEPOOL.get_mut().unwrap().palloc() }
}

fn pfree(page: Page) -> Result<(), VmError> {
    unsafe { PAGEPOOL.get_mut().unwrap().pfree(page) }
}


// -------------------------------------------------------------------

/// Out facing interface for physical pages. Automatically cleaned up
/// on drop. Intentionally does not impliment clone/copy/anything.
pub struct PhysPageExtent {
    head: Page,
    num: usize,
}

impl PhysPageExtent {
    pub fn start(&self) -> *mut usize {
        self.head.addr
    }

    pub fn end(&self) -> *mut usize {
        unsafe {
            self.head.addr.byte_add(self.num * PAGE_SIZE)
        }
    }
}

impl Drop for PhysPageExtent {
    fn drop(&mut self) {
        unsafe {
            match PAGEPOOL.get_mut().unwrap()
                .pfree_plural(self.head.addr, self.num) {
                    Ok(_) => {},
                    Err(e) => {panic!("Double palloc free! {:?}", e)}
            }
        }
    }
}

unsafe impl Send for PhysPageExtent {}


// VERY IMPORTANT: see top of module comment about deadlock safety
/// Should be one and only way to get physical pages outside of vm module/subsystem.
pub fn request_phys_page(num: usize) -> Result<PhysPageExtent, VmError>{
    let addr = unsafe {
        PAGEPOOL.get_mut().unwrap().palloc_plural(num)?
    };
    Ok(PhysPageExtent {
        head: Page::from(addr),
        num,
    })
}

pub fn test_phys_page() {
    {
        let _ = request_phys_page(1).unwrap();
        let _ = request_phys_page(2).unwrap();
    }
    let _ = request_phys_page(1).unwrap();
}
