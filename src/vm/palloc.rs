//! Physical page allocator
use crate::hw::param::*;
use crate::lock::mutex::Mutex;
use crate::vm::{Palloc, VmError};

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

impl Palloc for PagePool {
    /// Allocate page of physical memory by returning a pointer
    /// to the allocated page.
    fn palloc(&mut self) -> Result<Page, VmError> {
        let mut pool = self.pool.lock();
        match pool.free {
            None => Err(VmError::OutOfPages),
            Some(page) => pool.alloc_page(page),
        }
    }

    /// Not Implemented.
    fn pfree(&mut self, _size: usize) -> Result<(), VmError> {
        todo!()
    }
}

impl FreeNode {
    fn new(prev: *mut usize, next: *mut usize) -> Self {
        FreeNode { prev, next }
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
    fn write_free(&mut self, free_node: FreeNode) {
        self.write_prev(free_node.prev);
        self.write_next(free_node.next);
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
    fn read_free(&mut self) -> FreeNode {
        unsafe {
            FreeNode::new(
                self.addr.read_volatile() as *mut usize,
                self.addr.add(1).read_volatile() as *mut usize,
            )
        }
    }
}

impl Pool {
    /// Setup a doubly linked list of chunks from the bottom to top addresses.
    /// Assume chunk will generally be PAGE_SIZE.
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

        Pool {
            free: Some(free),
            bottom,
            top,
        }
    }

    /// Remove the current head of the doubly linked list and replace it
    /// with the next free page in the list.
    /// If this is the last free page in the pool, set the free pool to None
    /// in order to trigger the OutOfPages error.
    fn alloc_page(&mut self, mut page: Page) -> Result<Page, VmError> {
        let free_node = page.read_free();
        let prev = free_node.prev;
        let next = free_node.next;

        if next.addr() == 0x0 {
            self.free = None;
        } else {
            let mut new = Page::from(next);
            new.write_prev(prev);
            self.free = Some(new);
        }

        if prev.addr() != 0x0 {
            Page::from(prev).write_next(next);
        }

        page.zero();
        Ok(page)
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
