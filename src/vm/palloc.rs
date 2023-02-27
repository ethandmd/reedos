//! Almost like a real physical page allocator but ...

//use core::array::from_fn;
//use core::cell::SyncUnsafeCell;

use crate::lock::mutex::Mutex;
use crate::hw::param::*;
//use crate::hw::riscv::read_tp;
use crate::vm::{Palloc, PallocError};


fn is_multiple(addr: usize, size: usize) -> bool {
    addr & (size - 1) == 0
}

/// Kernel page pool. Each hart has their own local pool, and there is 
/// a global pool should a given hart's local pool run dry. This local -
/// global design should reduce lock contention.
pub struct PagePool {
    pool: Mutex<Pool>, //[Mutex<Pool>; NHART + 1],
}

/// Characterizes any pool by tracking free pages.
struct Pool {
    free: Page,                 // Head of free page list (stored in the free pages).
    bottom: *mut usize,         // Min addr of this page allocation pool.
    top: *mut usize,            // Max addr of this page allocation pool.
}

struct FreeNode {
    prev: *mut usize,
    next: *mut usize,
}

// TODO: Add methods to manipulate this address without pub addr field.
#[derive(PartialEq, PartialOrd, Eq, Ord)]
#[repr(transparent)]
pub struct Page {
    pub addr: *mut usize,  // ptr to start of page.
}

impl Palloc for PagePool {
    fn palloc(&mut self, size: usize) -> Result<Page, PallocError> { 
        todo!()
    }

    fn pfree(&mut self, size: usize) -> Result<(), PallocError> { 
        todo!()
    }
}

impl FreeNode {
    fn new(prev: *mut usize, next: *mut usize) -> Self {
        FreeNode { prev, next }
    }
}

impl Page {
    fn new(addr: *mut usize) -> Self {
        Page { addr }
    }

    // Takes a free page and writes the previous free page's addr in
    // the first 8 bytes. Then writes the next free page's addr in the
    // following 8 bytes.
    fn write_free(&mut self, free_node: FreeNode) {
        unsafe {
            self.addr.write_volatile(free_node.prev.addr());
            self.addr.add(1).write_volatile(free_node.next.addr());
        }
    }

    fn read_free(&mut self) -> FreeNode {
        let is_free = unsafe { self.addr.read_volatile() };
        if is_free == 1 {
            panic!("Attempted to use allocated page as free page.");
        } else {
            unsafe {
                FreeNode::new(
                    self.addr.read_volatile() as *mut usize,
                    self.addr.add(1).read_volatile() as *mut usize,
                )
            }
        }
    }
}

impl Pool {
    fn new(bottom: *mut usize, top: *mut usize, chunk_size: usize) -> Self {
        // Set up head of the free list.
        let mut free = Page::new(bottom);
        let mut pa = bottom.map_addr(|addr| addr + chunk_size);
        let tmp = FreeNode::new(0x0 as *mut usize, pa); // First free page 'prev' == 0x0 => none.
        free.write_free(tmp);
        let last = top.map_addr(|addr| addr - chunk_size);
        // Init the remainder of the free list.
        while pa < top {
            let prev_pa = pa.map_addr(|addr| addr - chunk_size);

            let next_pa = if pa == last {
                0x0 as *mut usize
            } else {
                pa.map_addr(|addr| addr + chunk_size)
            };

            let mut tmp = Page::new(pa);
            tmp.write_free(FreeNode::new(prev_pa, next_pa));
            pa = pa.map_addr(|addr| addr + chunk_size); // Don't use next_pa. End of loop will fail.
        }
        
        Pool { free, bottom, top }
    }
}

impl PagePool {
    pub fn new(bottom: *mut usize, top: *mut usize) -> Self {
        assert!(is_multiple(bottom.addr(), PAGE_SIZE));
        assert!(is_multiple(top.addr(), PAGE_SIZE));
        
        // LEFT AS COMMENT FOR FUTURE REFERENCE:
        //let total_size = top.addr() - bottom.addr();
        //let local_size = total_size / (2 * NHART);
        //assert!(is_multiple(local_size, PAGE_SIZE));

        //let pool: [Mutex<Pool>; NHART + 1] = from_fn(|id| {
        //    let per_start = bottom.map_addr(|addr| addr + (local_size * id));
        //    let per_top = bottom.map_addr(|addr| addr + (local_size) * (id + 1));
        //    if id < NHART {
        //        Mutex::new(Pool::new(per_start, per_top))
        //    } else {
        //        Mutex::new(Pool::new(per_start, top))
        //    }
        //});
        let pool = Mutex::new(Pool::new(bottom, top, PAGE_SIZE));
        PagePool { pool }
    }
}
