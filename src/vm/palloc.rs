use crate::alloc::Kalloc;
use crate::hw::riscv::read_mhartid;
use crate::lock::mutex::Mutex;
use crate::param::{NHART, PAGE_SIZE};

use core::alloc::{GlobalAlloc, Layout};
use core::array::from_fn;
use core::assert;
use core::ptr::null_mut;

pub struct Kpools {
    global: [Mutex<Kalloc>; NHART + 1],
    local_size: usize,
    start: usize,
    end: usize,
}

impl Kpools {
    /// Set up locked local pools per hart
    /// + locked global pool.
    ///
    /// Takes byte pointers defining the managed region. They must be page aligned.
    pub fn new(start: usize, end: usize) -> Self {
        assert!(PAGE_SIZE == 1 << 12, "Unexpected page size in Kpools.");
        assert!(
            start & !(4096 - 1) != 0,
            "Kpools managed region start isn't page aligned."
        );
        assert!(
            end & !(4096 - 1) != 0,
            "Kpools managed region end isn't page aligned."
        );

        let local_size = (end - start) / 2 * NHART;
        // Round down to pagesize
        let local_size = local_size >> 12;
        let local_size = local_size << 12;

        let global: [Mutex<Kalloc>; NHART + 1] = from_fn(|id| {
            let local_start = start + local_size * id;
            let global_start = start + (local_size * (NHART + 1));
            if id < NHART {
                return Mutex::new(Kalloc::new(local_start, local_start + local_size));
            } else {
                return Mutex::new(Kalloc::new(global_start, end));
            }
        });

        // shorthand binds to variables with the same name
        Kpools {
            global,
            local_size,
            start,
            end,
        }
    }

    /// Either gives you a pointer to a new page (zeroed), or null.
    pub fn palloc(&mut self) -> *mut u8 {
        let id = read_mhartid();
        let page_req: Layout = Layout::from_size_align(PAGE_SIZE, PAGE_SIZE).unwrap();

        // try local pool first
        let local = self.global[id as usize].lock();
        unsafe {
            let ret = (*local).alloc(page_req);
            if ret != null_mut::<u8>() {
                ret.write_bytes(0, PAGE_SIZE);
                return ret;
                // drops local mutex
            } else {
                drop(local); // we don't need it, this is faster. Could wait for the scope to drop
                let global_pool = self.global[NHART].lock();
                let ret = (*global_pool).alloc(page_req);
                if ret != null_mut::<u8>() {
                    ret.write_bytes(0, PAGE_SIZE);
                    return ret;
                    // drops global
                } else {
                    return null_mut::<u8>();
                }
            }
        }
    }

    /// frees a page given by palloc
    pub fn free(&mut self, ptr: *mut u8) {
        let page_req: Layout = Layout::from_size_align(PAGE_SIZE, PAGE_SIZE).unwrap();

        let offset = (ptr as usize) - self.start;
        let index = offset / self.local_size;
        let pool = self.global[index].lock();
        unsafe {
            (*pool).dealloc(ptr, page_req);
        }
    }
}
