//! Physical page allocator
use crate::hw::param::*;
use crate::lock::mutex::Mutex;
use crate::vm::VmError;

// For safety reasons, this module and all submodules MUST not rely on
// any kind of dynamic allocation in the rust sense. This would cause
// a dependency cycle, which is bad enough, but more importantly could
// lead to deadlock betwen the palloc lock and the global alloc
// lock. This warning is repeated elsewhere

/// Utility function, primarily used to check if addresses are page aligned.
fn is_multiple(addr: usize, size: usize) -> bool {
    addr & (size - 1) == 0
}

/// Kernel page pool.
pub struct PagePool {
    pool: Mutex<Pool>, //[Mutex<Pool>; NHART + 1],
}

/// Characterizes a page pool by tracking free pages with a double linked list.
struct Pool {
    free: Option<Page>, // Head of free page list (stored in the free pages).
    bottom: *mut usize, // Min addr of this page allocation pool.
    top: *mut usize,    // Max addr of this page allocation pool.
}

/// Convenience struct to read a free page like a doubly linked list.
struct FreeNode {
    prev: *mut usize,
    next: *mut usize,
}

/// Abstraction of a physical page of memory.
// TODO: Add methods to manipulate this address without pub addr field.
#[derive(Copy, Clone)]
#[repr(transparent)]
pub struct Page {
    pub addr: *mut usize, // ptr to first byte of page.
}

impl FreeNode {
    fn new(prev: *mut usize, next: *mut usize) -> Self {
        FreeNode { prev, next }
    }
}

impl PagePool {
    /// Allocate page of physical memory by returning a pointer
    /// to the allocated page from the doubly linked free list.
    pub fn palloc(&mut self) -> Result<Page, VmError> {
        let mut pool = self.pool.lock();
        match pool.free {
            None => Err(VmError::OutOfPages),
            Some(page) => match pool.alloc_pages(page, 1) {
                Err(_) => Err(VmError::OutOfPages),
                Ok(ptr) => Ok(ptr),
            },
        }
    }

    /// Free a page of physical memory by inserting into the doubly
    /// linked free list in order.
    pub fn pfree(&mut self, page: Page) -> Result<(), VmError> {
        if !is_multiple(page.addr.addr(), PAGE_SIZE) {
            panic!("Free page addr not page aligned.")
        }

        let mut pool = self.pool.lock();
        pool.free_pages(page, 1);
        Ok(())
    }

    pub fn palloc_plural(&mut self, num_pages: usize) -> Result<*mut usize, VmError> {
        assert!(num_pages != 0, "tried to allocate zero pages");
        let mut pool = self.pool.lock();
        match pool.free {
            None => Err(VmError::OutOfPages),
            Some(page) => match pool.alloc_pages(page, num_pages) {
                Err(_) => Err(VmError::OutOfPages),
                // ^ TODO consider partial allocations?
                Ok(ptr) => Ok(ptr.addr),
            },
        }
    }

    pub fn pfree_plural(&mut self, page: *mut usize, num_pages: usize) -> Result<(), VmError> {
        assert!(num_pages != 0, "tried to allocate zero pages");
        if !is_multiple(page.addr(), PAGE_SIZE) {
            panic!("Free page addr not page aligned.")
        }

        let mut pool = self.pool.lock();
        pool.free_pages(Page::from(page), num_pages);
        Ok(())
    }
}

/// Create a new page from a physical address.
impl From<*mut usize> for Page {
    fn from(addr: *mut usize) -> Self {
        Page { addr }
    }
}

impl Page {
    /// Create a new page from a physical address.
    /// Zero the addr + 4096 bytes before returning.
    // Watchout, this zeroes new pages.
    // If you don't want to zero, use From<T>.
    fn new(addr: *mut usize) -> Self {
        unsafe {
            addr.write_bytes(0, 512);
        }
        Page { addr }
    }

    /// Zero a page.
    // 'size' is in bytes. write_bytes() takes count * size_of::<T>() in bytes.
    // Since usize is 8 bytes, we want to zero out the page. Aka zero 512 PTEs.
    fn zero(&mut self) {
        unsafe {
            self.addr.write_bytes(0, 512);
        }
    }

    /// Write pointers to the previous and next pointers of the doubly
    /// linked list to this page. We use the first 8 bytes of the page to
    /// store a ptr to the previous page, and the second 8 bytes to
    /// store a ptr to the next page.
    // Takes a free page and writes the previous free page's addr in
    // the first 8 bytes. Then writes the next free page's addr in the
    // following 8 bytes.
    fn write_free(&mut self, prev: *mut usize, next: *mut usize) {
        self.write_prev(prev);
        self.write_next(next);
    }

    /// Write the next pointer of the doubly linked list to this page.
    fn write_next(&mut self, next: *mut usize) {
        unsafe {
            self.addr.add(1).write_volatile(next.addr());
        }
    }

    /// Write the previous pointer of the doubly linked list to this page.
    fn write_prev(&mut self, prev: *mut usize) {
        unsafe {
            self.addr.write_volatile(prev.addr());
        }
    }

    /// Read the prev, next pointers of a page in the free list.
    fn read_free(&mut self) -> (*mut usize, *mut usize) {
        unsafe {
            (
                self.addr.read_volatile() as *mut usize,
                self.addr.add(1).read_volatile() as *mut usize,
            )
        }
    }
}

enum PageError {
    NoGap,
    // maybe more?
}

impl Pool {
    /// Setup a doubly linked list of chunks from the bottom to top addresses.
    /// Assume chunk will generally be PAGE_SIZE.
    fn new(bottom: *mut usize, top: *mut usize, chunk_size: usize) -> Self {
        // Set up head of the free list.
        let mut free = Page::new(bottom);
        let mut pa = bottom.map_addr(|addr| addr + chunk_size);
        //let tmp = FreeNode::new(0x0 as *mut usize, pa); // First free page 'prev' == 0x0 => none.
        free.write_free(core::ptr::null_mut::<usize>(), pa);
        let last = top.map_addr(|addr| addr - chunk_size);
        // Init the remainder of the free list.
        while pa < top {
            let prev_pa = pa.map_addr(|addr| addr - chunk_size);

            let next_pa = if pa == last {
                core::ptr::null_mut::<usize>()
            } else {
                pa.map_addr(|addr| addr + chunk_size)
            };

            let mut tmp = Page::new(pa);
            tmp.write_free(prev_pa, next_pa);
            pa = pa.map_addr(|addr| addr + chunk_size); // Don't use next_pa. End of loop will fail.
        }

        Pool {
            free: Some(free),
            bottom,
            top,
        }
    }

    // If this is the last free page in the pool, set the free pool to None
    // in order to trigger the OutOfPages error.
    fn alloc_pages(&mut self, mut page: Page, num_pages: usize) -> Result<Page, PageError> {
        let (prev, mut next) = page.read_free(); // prev is always 0x0
        let example_null = core::ptr::null_mut::<usize>();
        assert_eq!(prev, example_null);
        // we don't use prev after this point

        let mut start_region = page;
        // ^ the first page of a contigous free region, we will take
        // start_region through page (inclusive) on success

        while (page.addr.map_addr(|addr| addr - start_region.addr.addr())).addr() / 0x1000
            < num_pages - 1
        {
            // until it's big enough

            while next as usize == page.addr as usize + 0x1000
                && (page.addr as usize - start_region.addr as usize) / 0x1000 < num_pages - 1
            {
                // until its big enough or there was a gap
                page = Page::from(next);
                (_, next) = page.read_free();
                if next as usize == 0x0 {
                    return Err(PageError::NoGap);
                    // ran off the end
                }
            }

            if next as usize != page.addr as usize + 0x1000 {
                // too short!
                start_region = Page::from(next);
            }
        }

        // we found it

        // would love this as a match but no dice
        let (sr_prev, _) = start_region.read_free();
        if sr_prev == example_null {
            // this was the first chunk
            if next as usize == 0x0 {
                self.free = None;
            } else {
                // remember next here is next from page
                let mut next_page = Page::from(next);
                next_page.write_prev(example_null);
                self.free = Some(next_page);
            }
        } else {
            let before_region = sr_prev;
            // not first chunk in pool
            Page::from(before_region).write_next(next);
            Page::from(next).write_prev(before_region);
        }

        // we found it
        // zero them all out
        let mut cur = start_region;
        while cur.addr as usize <= page.addr as usize {
            cur.zero();
            cur = Page::from(cur.addr.map_addr(|addr| addr + 0x1000));
        }

        Ok(start_region)
    }

    fn free_pages(&mut self, mut page: Page, num_pages: usize) {
        assert!(num_pages != 0, "Tried to free zero pages");
        let example_null = core::ptr::null_mut::<usize>();

        let mut region_end = Page::from(page.addr.map_addr(|addr| addr + (num_pages - 1) * 0x1000));
        let stop = region_end.addr.map_addr(|addr| addr + 0x1000);
        let mut prev_page: Option<Page> = None;
        let mut curr_page = page;
        while curr_page.addr < stop {
            curr_page.zero();
            let next_page = Page::from(curr_page.addr.map_addr(|addr| addr + 0x1000));
            match prev_page {
                None => {
                    curr_page.write_next(next_page.addr);
                }
                Some(mut prev) => {
                    curr_page.write_prev(prev.addr);
                    prev.write_next(curr_page.addr);
                }
            }
            (prev_page, curr_page) = (Some(curr_page), next_page);
        }
        // zeroed and internally linked

        match self.free {
            Some(mut head) => {
                // special case, insert at beginning
                if head.addr > region_end.addr {
                    head.write_prev(region_end.addr);
                    region_end.write_next(head.addr);
                    page.write_prev(example_null);
                    self.free = Some(page);
                } else {
                    // will insert after insert_location
                    let mut head_next = head.read_free().1;
                    while head_next != example_null && head_next < region_end.addr {
                        head = Page::from(head_next);
                        head_next = head.read_free().1;
                    }

                    if head_next == example_null {
                        // insert at the end
                        region_end.write_next(example_null);
                        page.write_prev(head.addr);
                        head.write_next(page.addr);
                    } else {
                        head.write_next(page.addr);
                        page.write_prev(head.addr);
                        region_end.write_next(head_next);
                        Page::from(head_next).write_prev(region_end.addr);
                    }
                }
            }
            None => {
                page.write_prev(example_null);
                region_end.write_next(example_null);
                self.free = Some(page);
            }
        }
    }
}

impl PagePool {
    /// Create a new pool within a mutex spinlock.
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

