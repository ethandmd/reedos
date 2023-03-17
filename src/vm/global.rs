use crate::param::PAGE_SIZE;
use crate::vm::palloc::PagePool;
use crate::vm::vmalloc::{Kalloc, MAX_CHUNK_SIZE};
/// Global allocator on top of vmalloc and palloc
use core::alloc::{GlobalAlloc, Layout};
use core::cell::UnsafeCell;

pub struct Galloc {
    pool: *mut PagePool,
    small_pool: UnsafeCell<Kalloc>,
}

impl Galloc {
    pub fn new(pool: &mut PagePool) -> Self {
        let small_pool_start = pool
            .palloc()
            .expect("Could not initalize GlobalAlloc small pool");
        Galloc {
            pool,
            small_pool: UnsafeCell::new(Kalloc::new(small_pool_start)),
        }
    }
}

impl Drop for Galloc {
    fn drop(&mut self) {
        panic!("Dropped the general allocator")
    }
}

/// Returns the number of pages to request from the page allocator or
/// zero to request from the sub-page allocator
fn decide_internal_scheme(layout: Layout) -> usize {
    match layout.size() {
        0 => {
            panic!("Tried zero size alloc")
        }
        1..=MAX_CHUNK_SIZE => {
            // try use small allocator
            match layout.align() {
                0..=8 => 0,
                _ => 1,
                // ^ alignment too large for Kalloc, round up to a page
            }
        }
        req_size => {
            // use page allocator
            (req_size + PAGE_SIZE - 1) / PAGE_SIZE
        }
    }
}

unsafe impl GlobalAlloc for Galloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if layout.align() > PAGE_SIZE {
            panic!("Page+ alignemnt requested in alloc");
        }

        let num_pages = decide_internal_scheme(layout);

        if num_pages == 0 {
            match (*self.small_pool.get()).alloc(layout.size()) {
                Ok(ptr) => ptr as *mut u8,
                Err(e) => {
                    panic!("Small allocation failed {:?}", e)
                }
            }
        } else {
            match (*self.pool).palloc_plural(num_pages) {
                Ok(ptr) => ptr as *mut u8,
                Err(e) => {
                    panic!("Page allocation failed {:?}", e)
                }
            }
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        if layout.align() > PAGE_SIZE {
            panic!("Page+ alignemnt requested in dealloc");
        }

        let num_pages = decide_internal_scheme(layout);

        if num_pages == 0 {
            (*self.small_pool.get()).free(ptr as *mut usize)
        } else {
            match (*self.pool).pfree_plural(ptr as *mut usize, num_pages) {
                Ok(_) => {}
                Err(e) => {
                    panic!("Page deallocation failed {:?}", e)
                }
            }
        }
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        let out = self.alloc(layout);

        let num_pages = decide_internal_scheme(layout);
        if num_pages == 0 {
            out.write_bytes(0, layout.size());
            out
        } else {
            // palloc already zeros
            out
        }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        // TODO improve
        let out = self.alloc(Layout::from_size_align(new_size, layout.align()).unwrap());
        core::intrinsics::copy_nonoverlapping(ptr, out, layout.size());
        self.dealloc(ptr, layout);
        out
    }
}
