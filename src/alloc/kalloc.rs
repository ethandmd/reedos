/// This is the usable wrapper for page alloc
///
///
/// TODO
///
/// do the rest of the globallAlloc stuff
/// make header.next() fail if the next thing would be in a different page
/// impliment a a page-agnostic wrapper

use core::alloc::*;
use core::mem;

const PAGE_SIZE: usize = 4096;
    
/// store the overhead info for a chunk
///
/// This is directly before the chunk allocated for user memory that
/// it corresponds to
///
/*
 * this is not currently true
 * 
/// next and prev are only meaningful if this chunk is both valid and
/// free. In that case they link in a circular doubly linked list to
/// the other free chunks of the pool. If this chunk is in use, then
/// the next chunk can be found by using the offset based on the size
/// in the header.
*/
struct k_chunk_header {
    size: usize,		// includes this header
    layout: Layout,
    // next: *mut k_chunk_header,
    // prev: *mut k_chunk_header,
    is_free: bool,
}

impl k_struct_header {
    fn init_free(&mut self, size: usize) {
	self.size = size;
	self.is_free = true;
    }

    fn size(&self) -> usize {
	self.size.clone()
    }

    fn layout(&self) -> Layout {
	self.layout.clone()
    }

    fn is_free(&self) -> bool {
	self.is_free.clone()
    }

    fn usable_size(&self) -> usize {
	self.size() - mem::size_of<Self>()
    }

    fn set_is_free(&mut self, free: bool) {
	self.is_free = free;
    }

    fn set_usable_size(&mut self, size: usize) {
	self.size = size + mem::size_of<Self>();
    }

    fn set_layout(&mut self, layout: Layout) {
	self.layout = layout;
    }

    fn user_data(&self) -> *mut u8 {
	(&self as *mut u8).offset(1)
    }

    //TODO this should be an option, as it might fail. How can we catch that?
    fn next(&self) -> *mut k_chunk_header {
	(&self as *mut u8).byte_offset(self.size())
    }

    fn attempt_merge(&mut self) {
	let next_chunk = self.next();
	if next_chunk.is_free() {
	    self.set_usable_size(self.usable_size() + next_chunk.size());
	}
    }
}

struct k_chunk_iter {
    current: *mut k_chunk_header,
    offset: usize,
}

impl Iterator for k_chunk_iter {
    type Item = k_chunk_header;

    fn next (&self) -> Option<Self::Item> {
	if offset == PAGE_SIZE {
	    None
	} else {
	    let hold = self.current;
	    self.current = self.current.byte_offset(self.current.size());
	    self.offset += self.current.size();
	    hold
	}
    }
}

pub struct kalloc {
    pool: *mut k_chunk_header, 		// exactly one page long
}

// gives *MUTABLE REFERENCES*
impl IntoIter for kalloc {
    type Item = &mut k_chunk_header;
    type IntoIter = k_chunk_iter;

    fn into_iter(self) -> Self::IntoIter {
	k_chunk_iter {
	    current: pool,
	    offset: 0,
	};
    }
}

impl kalloc {
    fn new(page: *mut u8) -> Self {
	self.pool = page;
    }
    
    /// make a size request conform to it's matching alignment
    fn adjust_size_with_align(size: usize, align: &usize) -> usize {
	let mask: usize = align - 1;
	if (size & mask) {
	    // has low order bits
	    (size & !mask) + align
	} else {
	    // already aligned
	    size
	}
    }

    /// adjust a pointer forward until it matches alignment at least
    fn adjust_ptr_with_align(ptr: *mut u8, align: &usize) -> *mut u8 {
	let mask: usize = align - 1;
	let addr: usize = ptr.addr();
	if (addr & mask) {
	    // low order bits
	    pointer::with_addr((addr & !mask) + align)
	} else {
	    ptr
	}
    }
}

impl GlobalAlloc for kalloc {
    fn alloc(layout: Layout) -> *mut u8 {
	let internal_align = min(layout.align(), mem::size_of<k_chunk_header>());
	let internal_size = adjust_size_with_align(layout.size(), internal_size);

	for &mut chunk in self {
	    let useable_size = chunk.usable_size();
	    if (useable_size >= internal_size && chunk.is_free) {
		// this will work
		if usable_size >= 2 * internal_size {
		    // too big, we should only take what we need
		    let new_chunk: *mut k_chunk_header = chunk.byte_offset(internal_size);
		    new_chunk.init_free(usable_size - internal_size);
		    new_chunk.attempt_merge();
		    // does not set layout of new chunk

		    chunk.set_size(internal_size);
		    chunk.set_free(false);
		    chunk.set_layout(layout);
		    return adjust_ptr_with_align(chunk.user_data(), internal_align);
		} else {
		    // this is close enough, just grab it
		    chunk.set_is_free(false);
		    chunk.set_layout(layout);
		    return adjust_ptr_with_align(chunk.user_data(), internal_align);
		}
	    } 
	}
    }

    //dealloc, realloc, alloc_with_zero, see docs
}

