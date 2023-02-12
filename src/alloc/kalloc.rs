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

// util page stuff
fn same_page(a: usize, b: usize) -> bool {
    a & !(PAGE_SIZE - 1) == b & !(PAGE_SIZE - 1)
}

/// store the overhead info for a chunk
///
/// This is directly before the chunk allocated for user memory that
/// it corresponds to
struct k_chunk_header {
    size: usize,		// includes this header
    layout: Layout,
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

    // pointer to the next header, or None. See kalloc struct for when this could fail
    fn next(&self) -> Option<*mut k_chunk_header> {
	let next = (&self as *mut k_chunk_header).byte_offset(self.size());
	if same_page(&self as usize, next as usize) {
	    Some(next)
	} else {
	    None
	}
    }

    fn attempt_merge(&mut self) {
	let pos_next = self.next();
	match pos_next {
	    Some(next) => {
		if next_chunk.is_free() {
		    self.set_usable_size(self.usable_size() + next_chunk.size());
		} else {
		    // do nothing
		}
	    },
	    None => {
		// do nothing
	    }
	}
    }

    fn matches_layout(layout: &Layout) -> bool {
	self.layout() == layout
    }
}

struct k_chunk_iter {
    current: Option<*mut k_chunk_header>,
}

impl Iterator for k_chunk_iter {
    type Item = k_chunk_header;

    fn next (&mut self) -> Option<Self::Item> {
	match self.current {
	    None => {
		None
	    },
	    Some(chunk) => {
		self.current = self.current.next();
		Some(chunk)
	    }
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
	    current: Some(pool),
	};
    }
}

impl kalloc {
    fn new(page: *mut u8) -> Self {
	self.pool = page as *mut k_chunk_header;
    }

    // TODO this and next can be replaced with stuff from core::pointer I think
    
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
    fn alloc(&self, layout: Layout) -> *mut u8 {
	let internal_align = min(layout.align(), mem::size_of<k_chunk_header>());
	let internal_size = adjust_size_with_align(layout.size(), internal_align);

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
	return Null;		// just means this page can't fit it, not a user level failure yet.
    }

    //dealloc, realloc, alloc_with_zero, see docs
    fn dealloc(&self, ptr: *mut u8, layout: Layout) {
	let previous: Option<*mut k_chunk_header> = None;
	for &mut chunk in self {
	    if chunk.matches_layout(layout) {
		// this is it
		chunk.set_is_free(true);
		chunk.attempt_merge();
		if previous != None && previous.is_free() {
		    previous.unwrap().attempt_merge();
		}
		return;
	    } else {
		// keep moving
		previous = Some(chunk as *mut k_chunk_header);
	    }
	}
	panic!("Dealloc Failure: Chunk not found. Have you changed your pointer or layout? Did you dealloc something from a different page?");
    }

    fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
	let ptr = self.alloc(layout);
	for i in 0..layout.size() {
	    ptr.byte_offset(i).write(0);
	}
	ptr
    }

    /// might work, but unsable and incomplete atm.
    ///
    /// highly suggest using alloc and dealloc yourself if you are moving large amounts of data
    fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) {
	let previous: Option<*mut k_chunk_header> = None;
	let internal_align = min(layout.align(), mem::size_of<k_chunk_header>());
	let internal_size = adjust_size_with_align(new_size, internal_align);
	
	for &mut chunk in self {
	    if chunk.matches_layout(layout) {
		// this is it
		chunk.set_is_free(true);
		chunk.attempt_merge();
		if previous != None && previous.unwrap().is_free() {
		    let p_chunk = previous.unwrap();
		    p_chunk.attempt_merge();
		    if p_chunk.usable_size() > internal_size {
			// we can use prev: copy and be wary of
			// overlap. we are writing forward from source
			// to dest, and dest comes before source, so
			// we should be fine
			p_chunk.set_is_free(false);
			let dest = p_chunk.user_data() as *mut u8;
			let src = chunk.user_data() as *mut u8;
			for off in 0..chunk.usable_size() { 
			    dest.byte_offset(off).write(
				src.byte_offset(off).read());
			}
			p_chunk.set_layout(layout.with_size(new_size));
			return p_chunk.user_data();
		    }
		} else if chunk.usable_size() > internal_size {
		    // prev wasn't free, didn't exist, or wasn't
		    // big enough. Next we can try to see if the
		    // newly (maybe) merged chunk we just "freed"
		    // is big enough.
		    chunk.set_is_free(false);
		    chunk.set_layout(layout.with_size(new_size));
		    return chunk.user_data();
		} else {
		    // neither prev nor the chunk we freed worked, just straight alloc

		    // TODO this retraverses part of the array. fix it.
		    //
		    // also calling out to alloc is a little
		    // risky, as currently chunk is marked as free
		    // but we have yet to move that data we need
		    // out of it. Currently it should be fine, but
		    // in the future, this should not call out to
		    // general alloc, as we don't guarentee that
		    // alloc won't do wacky stuff to try to find
		    // space, possibly clobbering the data we need
		    //
		    // also this may fail, if the current page
		    // doesn't have space for it, as it will not
		    // allocate nor search for other pools to
		    // relocate into
		    let dest = self.alloc(layout.with_size(new_size));
		    let src = chunk.user_data();
		    for off in 0..chunk.usable_size() {
			dest.byte_offset(off).write(
			    src.byte_offset(off).read());
		    }
		    return dest;
		} 
	    }
	}
	panic!("Realloc Failure: Chunk not found. Have you changed your pointer or layout? Did you dealloc something from a different page?")
    }
}

