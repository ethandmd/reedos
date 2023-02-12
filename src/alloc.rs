/// This is the usable wrapper for page alloc
///
///
/// TODO
///
/// do the rest of the globallAlloc stuff
/// make header.next() fail if the next thing would be in a different page
/// impliment a a page-agnostic wrapper

use core::alloc::*;
use core::mem::size_of;

use core::iter::IntoIterator;
use core::cmp::max;
use core::ptr::null_mut;


extern "C" {
    static _heap_start: usize;
    static _heap_end: usize;
}

fn out_of_bounds(addr: usize) -> bool {
    unsafe {
	addr < _heap_start || addr >= _heap_end
    }
}

/// store the overhead info for a chunk
///
/// This is directly before the chunk allocated for user memory that
/// it corresponds to
struct KChunkHeader {
    size: usize,		// includes this header
    layout: Layout,
    is_free: bool,
    alignment_offset: usize, 	// number of bytes between end of header and begining of user data
}

impl KChunkHeader {
    fn init_free(&mut self, size: usize) {
	self.size = size;
	self.is_free = true;
	self.alignment_offset = 0;
    }

    fn alignment_offset(&self) -> usize {
	self.alignment_offset.clone()
    }

    // total size with header, padding, and data
    fn size(&self) -> usize {
	self.size.clone()
    }

    // just padding and data
    fn size_with_padding(&self) -> usize {
	self.size() - size_of::<Self>()

    }

    // just the size of user data
    fn user_size(&self) -> usize {
	self.size() - size_of::<Self>() - self.alignment_offset()
    }
    
    fn layout(&self) -> Layout {
	self.layout.clone()
    }

    fn is_free(&self) -> bool {
	self.is_free.clone()
    }

    fn set_is_free(&mut self, free: bool) {
	self.is_free = free;
    }

    fn set_user_size(&mut self, user_size: usize) {
	self.size = user_size + size_of::<Self>() + self.alignment_offset();
    }

    fn set_padded_size(&mut self, padded_size: usize) {
	self.size = padded_size + size_of::<Self>();
    }
    
    fn set_layout(&mut self, layout: Layout) {
	self.layout = layout;
    }

    fn set_alignment_offset(&mut self, offset: usize) {
	self.alignment_offset = offset;
    }

    // kind of contrary to the spirit of rust type system, but the
    // only way I can get it to shut up on compile
    fn start_of_padding(&mut self) -> *mut u8 {
	(((self as *mut KChunkHeader) as usize) + size_of::<Self>()) as *mut u8
    }
 
    fn user_data(&mut self) -> *mut u8 {
	(((self as *mut KChunkHeader) as usize) + size_of::<Self>() + self.alignment_offset()) as *mut u8
    }

    // pointer to the next header, or None. See kalloc struct for when this could fail
    unsafe fn next_chunk(&mut self) -> Option<*mut KChunkHeader> {
	let next = (self as *mut KChunkHeader).byte_offset(self.size() as isize);
	if out_of_bounds(next as usize) {
	    None
	} else {
	    Some(next)
	}
    }

    unsafe fn attempt_merge(&mut self) {
	let pos_next = self.next_chunk();
	match pos_next {
	    Some(next_chunk) => {
		if (*next_chunk).is_free() {
		    self.set_user_size(self.user_size() + (*next_chunk).size());
		} else {
		    // do nothing
		}
	    },
	    None => {
		// do nothing
	    }
	}
    }

    fn matches_layout(&self, layout: &Layout) -> bool {
	self.layout() == *layout
    }
}

struct KChunkIter {
    current: Option<*mut KChunkHeader>,
}

impl Iterator for KChunkIter {
    type Item = *mut KChunkHeader;

    fn next (&mut self) -> Option<Self::Item> {
	match self.current {
	    None => {
		None
	    },
	    Some(chunk) => {
		unsafe {
		    self.current = (*self.current.unwrap()).next_chunk();
		    Some(chunk)
		}
	    }
	}
    }
}

pub struct Kalloc {
    pool: *mut KChunkHeader, 		// size defined by linker script, see _heap_size
}

// gives *MUTABLE REFERENCES*
impl IntoIterator for Kalloc {
    type Item = *mut KChunkHeader;
    type IntoIter = KChunkIter;

    fn into_iter(self) -> Self::IntoIter {
	KChunkIter {
	    current: Some(self.pool),
	}
    }
}

impl Kalloc {
    fn new(pool: *mut u8) -> Self {
	Kalloc {
	    pool: pool as *mut KChunkHeader,
	}
    }

    fn iter(&self) -> KChunkIter {
	KChunkIter {
	    current: Some(self.pool),
	}
    }
    
    fn mut_iter(&mut self) -> KChunkIter {
	KChunkIter {
	    current: Some(self.pool),
	}
    }

    // TODO this and next can be replaced with stuff from core::pointer I think
    
    /// make a size request conform to it's matching alignment
    pub fn adjust_size_with_align(size: usize, align: &usize) -> usize {
	let mask: usize = align - 1;
	if (size & mask) != 0{
	    // has low order bits
	    (size & !mask) + align
	} else {
	    // already aligned
	    size
	}
    }

    /// adjust a pointer forward until it matches alignment at least
    pub fn adjust_ptr_with_align(ptr: *mut u8, align: &usize) -> (*mut u8, usize) {
	let mask: usize = align - 1;
	let addr: usize = ptr as usize;
	if (addr & mask) != 0 {
	    // low order bits
	    (((addr & !mask) + align) as *mut u8,
	     align - (addr & mask))
	} else {
	    (ptr, 0)
	}
    }

}

unsafe impl GlobalAlloc for Kalloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
	let internal_align = max(layout.align(), size_of::<KChunkHeader>());
	let internal_size = Kalloc::adjust_size_with_align(layout.size(), &internal_align);

	for cptr in *self {
	    let chunk: &mut KChunkHeader = &mut *cptr;
	    let padded_size = chunk.size_with_padding();
	    if padded_size >= internal_size && chunk.is_free() {
		// this will work
		if padded_size >= 2 * internal_size {
		    // too big, we should only take what we need
		    let new_chunk: *mut KChunkHeader = cptr.byte_offset(internal_size as isize);
		    (*new_chunk).init_free(padded_size - internal_size);
		    // does not set layout of new chunk

		    (*chunk).set_padded_size(internal_size);
		    (*chunk).set_is_free(false);
		    (*chunk).set_layout(layout);
		    let ptr_and_offset = Kalloc::adjust_ptr_with_align(chunk.user_data(), &internal_align);
		    chunk.set_alignment_offset(ptr_and_offset.1);
		    return ptr_and_offset.0;
		} else {
		    // this is close enough, just grab it
		    chunk.set_is_free(false);
		    chunk.set_layout(layout);
		    let ptr_and_offset = Kalloc::adjust_ptr_with_align(chunk.user_data(), &internal_align);
		    chunk.set_alignment_offset(ptr_and_offset.1);
		    return ptr_and_offset.0;
		}
	    } 
	}
	return null_mut::<u8>();		// Can't find any space
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
	let mut previous: Option<*mut KChunkHeader> = None;
	for cptr in *self {
	    let chunk: &mut KChunkHeader = &mut *cptr;
	    if (*chunk).matches_layout(&layout) && (*chunk).user_data() == ptr {
		// this is it
		chunk.set_is_free(true);
		chunk.attempt_merge();
		if previous != None && (*previous.unwrap()).is_free() {
		    (*previous.unwrap()).attempt_merge();
		}
		return;
	    } else {
		// keep moving
		previous = Some(chunk as *mut KChunkHeader);
	    }
	}
	panic!("Dealloc Failure: Chunk not found. Have you changed your pointer or layout?");
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
	let ptr = self.alloc(layout);
	ptr.write_bytes(0, layout.size());
	ptr
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
	let mut previous: Option<*mut KChunkHeader> = None;
	let internal_align = max(layout.align(), size_of::<KChunkHeader>());
	let internal_size = Kalloc::adjust_size_with_align(new_size, &internal_align);
	
	for cptr in *self {
	    let chunk = &mut *cptr;
	    if chunk.matches_layout(&layout) && (*chunk).user_data() == ptr {
		// this is it
		chunk.set_is_free(true);
		chunk.attempt_merge();
		if previous != None && (*previous.unwrap()).is_free() {
		    let p_chunk = &mut *previous.unwrap();
		    p_chunk.attempt_merge();
		    if p_chunk.size_with_padding() > internal_size {
			// we can use prev: copy and be wary of
			// overlap. we are writing forward from source
			// to dest, and dest comes before source, so
			// we should be fine
			p_chunk.set_is_free(false);
			p_chunk.set_layout(Layout::from_size_align(new_size, layout.align()).unwrap());
			
			let ptr_and_offset = Kalloc::adjust_ptr_with_align(p_chunk.start_of_padding(), &internal_align);
			p_chunk.set_alignment_offset(ptr_and_offset.1);

			let dest = p_chunk.user_data();
			let src = chunk.user_data();
			for off in 0..chunk.user_size() { 
			    dest.byte_offset(off as isize).write(
				src.byte_offset(off as isize).read());
			}
			return ptr_and_offset.0;
		    }
		} else if chunk.size_with_padding() > internal_size {
		    // prev wasn't free or didn't exist. But this
		    // chunk that has been freed and merged is big
		    // enough. We can avoid any data movement
		    chunk.set_is_free(false);
		    chunk.set_layout(Layout::from_size_align(new_size, layout.align()).unwrap());
		    return chunk.user_data();
		} else {
		    // neither prev nor the chunk we freed worked, just straight alloc

		    // TODO this retraverses part of the array. fix it.

		    chunk.set_is_free(false); // avoid clobbering side effects of future alloc optimizations
		    
		    let dest = self.alloc(Layout::from_size_align(new_size, layout.align()).unwrap());
		    if dest == null_mut::<u8>() {
			panic!("Realloc Failure: Couldn't allocate a new chunk.");
		    }
		    let src = chunk.user_data();
		    for off in 0..chunk.user_size() {
			dest.byte_offset(off as isize).write(
			    src.byte_offset(off as isize).read());
		    }
		    return dest;
		} 
	    }
	}
	panic!("Realloc Failure: Chunk not found. Have you changed your pointer or layout?")
    }
}

