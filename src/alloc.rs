/// This is the usable wrapper for alloc. It implements a general
/// allocate that could be used for pages or for small scale kernel
/// stuff. See Kalloc::new.
///
///
/// TODO fix the data path for things like start and end, I shouldn't
/// have to pass them around, but I also don't want them inside the
/// headers themselves.
///
/// Externally, you should only use the Kalloc new and GlobalAlloc
/// functions. The rest may have assumptions that are not enforced by
/// the compiler
use core::alloc::*;
use core::cell::UnsafeCell;
use core::mem::size_of;

use core::cmp::max;
use core::ptr::null_mut;

/// General util function. Enforces the same meaning on start and end
/// everywhere.
fn out_of_bounds(addr: usize, heap_start: usize, heap_end: usize) -> bool {
    addr < heap_start || addr >= heap_end
}

/// Stores the overhead info for a chunk.
///
/// This is directly before the chunk allocated for user memory that
/// it corresponds to.
#[derive(Debug)]
pub struct KChunkHeader {
    size: usize,             // Size of the chunk including this header
    layout: Layout,          // What alloc/realloc call was this a response to?
    is_free: bool,           // Is this chunk in use or not?
    alignment_offset: usize, // Number of bytes between end of header and beginning of user data
}

impl KChunkHeader {
    /// setup a header for a chunk of this size (including header)
    /// that is not in use.
    fn init_free(&mut self, size: usize) {
        self.size = size;
        self.is_free = true;
        self.alignment_offset = 0;
    }

    /// How many bytes between the header and the user data?
    fn alignment_offset(&self) -> usize {
        self.alignment_offset
    }

    /// Total size of chunk with header, padding, and data
    fn size(&self) -> usize {
        self.size
    }

    /// Size of the usable part of the chunk, the padding + the user data
    fn size_with_padding(&self) -> usize {
        self.size() - size_of::<Self>()
    }

    /// The number of bytes the user can safely use. Does NOT include
    /// padding or header.
    fn user_size(&self) -> usize {
        self.size() - size_of::<Self>() - self.alignment_offset()
    }

    /// What call was this chunk grabbed in response to?
    fn layout(&self) -> Layout {
        self.layout
    }

    /// Is this chunk up for grabs?
    fn is_free(&self) -> bool {
        self.is_free
    }

    /// Update whether or not this chunk is in use.
    fn set_is_free(&mut self, free: bool) {
        self.is_free = free;
    }

    /// Update the user visible size of this chunk by resizing the chunk.
    fn set_user_size(&mut self, user_size: usize) {
        self.size = user_size + size_of::<Self>() + self.alignment_offset();
    }

    /// Update the usable size of this chunk by resizing.
    fn set_padded_size(&mut self, padded_size: usize) {
        self.size = padded_size + size_of::<Self>();
    }

    /// Set the fingerprint of the call that put this into use.
    fn set_layout(&mut self, layout: Layout) {
        self.layout = layout;
    }

    /// Set the padding between the header and the user data.
    fn set_alignment_offset(&mut self, offset: usize) {
        self.alignment_offset = offset;
    }

    /// Gives a pointer to the first byte of padding. Check the size yourself.
    ///
    /// TODO boy this is not rustish. No idea if that's fixable though.
    fn start_of_padding(&mut self) -> *mut u8 {
        (((self as *mut KChunkHeader) as usize) + size_of::<Self>()) as *mut u8
    }

    /// Give a pointer to the start of user data. Check the size yourself.
    fn user_data(&mut self) -> *mut u8 {
        (((self as *mut KChunkHeader) as usize) + size_of::<Self>() + self.alignment_offset())
            as *mut u8
    }

    /// Returns a pointer to the next header or None. Takes the info
    /// about the heap it is in to make the call about if it is at the
    /// end of the header list. See file top comment.
    unsafe fn next_chunk(
        &mut self,
        heap_start: usize,
        heap_end: usize,
    ) -> Option<*mut KChunkHeader> {
        let next = (self as *mut KChunkHeader).byte_offset(self.size() as isize);
        if out_of_bounds(next as usize, heap_start, heap_end) {
            None
        } else {
            Some(next)
        }
    }

    /// Only safe on free chunks. Tries to merge this chunk with the
    /// next if it is also free. Takes bounds for safety, see file top
    /// comment.
    unsafe fn attempt_merge(&mut self, heap_start: usize, heap_end: usize) {
        let pos_next = self.next_chunk(heap_start, heap_end);
        match pos_next {
            Some(next_chunk) => {
                if (*next_chunk).is_free() {
                    self.set_user_size(self.user_size() + (*next_chunk).size());
                } else {
                    // do nothing
                }
            }
            None => {
                // do nothing
            }
        }
    }

    /// Does this chunk match this caller fingerprint?
    fn matches_layout(&self, layout: &Layout) -> bool {
        self.layout() == *layout
    }
}

// We want to be able to for loop over these headers, as a painless way to deal with them
struct KChunkIter {
    start: usize,                       // heap start for this run of chunks
    end: usize,                         // heap end for this run of chunks
    current: Option<*mut KChunkHeader>, // where are we in the run, the value returned by the next call to next()
}

impl Iterator for KChunkIter {
    type Item = *mut KChunkHeader;

    fn next(&mut self) -> Option<Self::Item> {
        match self.current {
            None => None,
            Some(chunk) => unsafe {
                self.current = (*self.current.unwrap()).next_chunk(self.start, self.end);
                Some(chunk)
            },
        }
    }
}

/// one allocation pool. Basically one "heap"
pub struct Kalloc {
    heap_start: usize,                   // start of the managed region of memory
    heap_end: usize,                     // end of the managed region of memory
    pool: UnsafeCell<*mut KChunkHeader>, // the actual data as it should be accessed
}

// Just so we can put it in a mutex
unsafe impl Sync for Kalloc {}

impl Kalloc {
    /// Make a new managed region given by the bounds. They are inclusive and exclusive respectively.
    pub fn new(start: usize, end: usize) -> Self {
        let first_chunk = start as *mut KChunkHeader;
        unsafe {
            (*first_chunk).init_free(end - start);
        }
        Kalloc {
            heap_start: start,
            heap_end: end,
            pool: UnsafeCell::new(start as *mut KChunkHeader),
        }
    }

    /// Iterate over the chunks of this region.
    fn mut_iter(&self) -> KChunkIter {
        unsafe {
            KChunkIter {
                start: self.heap_start,
                end: self.heap_end,
                current: Some(*self.pool.get()),
            }
        }
    }

    // TODO this and next can be replaced with stuff from core::pointer I think

    pub fn adjust_size_with_align(size: usize, align: usize) -> usize {
        let mask: usize = align - 1;
        if (size & mask) != 0 {
            // has low order bits
            (size & !mask) + align
        } else {
            // already aligned
            size
        }
    }

    /// Adjust a pointer forward until it matches alignment at least.
    ///
    /// Returns a tuple of the changed pointer and the number of bytes
    /// forward that it was moved
    pub fn adjust_ptr_with_align(ptr: *mut u8, align: usize) -> (*mut u8, usize) {
        let mask: usize = align - 1;
        let addr: usize = ptr as usize;
        if (addr & mask) != 0 {
            // low order bits
            (((addr & !mask) + align) as *mut u8, align - (addr & mask))
        } else {
            (ptr, 0)
        }
    }

    pub fn print_alloc(&self) {
        for cptr in self.mut_iter() {
            unsafe {
                let chunk: &KChunkHeader = &*cptr;
                print!("{:?}", *chunk);
            }
        }
    }
}

// Does what it says.
unsafe impl GlobalAlloc for Kalloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let internal_align = max(layout.align(), size_of::<KChunkHeader>());


        for cptr in self.mut_iter() {
            let chunk: &mut KChunkHeader = &mut *cptr;
            let aligned_ptr = Kalloc::adjust_ptr_with_align(
                chunk.start_of_padding(), internal_align).0 as usize;
            let aligned_ptr_size = aligned_ptr + layout.size();
            let required_size = aligned_ptr_size - chunk.start_of_padding() as usize;
            let padded_size = chunk.size_with_padding();
            if padded_size >= required_size && chunk.is_free() {
                // this will work
                if padded_size >= 2 * required_size {
                    // too big, we should only take what we need

                    // how many bytes to the next header
                    let skip: usize = required_size + size_of::<KChunkHeader>();

                    let new_chunk: *mut KChunkHeader = cptr.byte_offset(skip as isize);
                    (*new_chunk).init_free(padded_size - skip);
                    // does not set layout of new chunk

                    (*chunk).set_padded_size(required_size);
                    (*chunk).set_is_free(false);
                    (*chunk).set_layout(layout);
                    let ptr_and_offset =
                        Kalloc::adjust_ptr_with_align(chunk.user_data(), internal_align);
                    chunk.set_alignment_offset(ptr_and_offset.1);
                    return ptr_and_offset.0;
                } else {
                    // this is close enough, just grab it
                    chunk.set_is_free(false);
                    chunk.set_layout(layout);
                    let ptr_and_offset =
                        Kalloc::adjust_ptr_with_align(chunk.user_data(), internal_align);
                    chunk.set_alignment_offset(ptr_and_offset.1);
                    return ptr_and_offset.0;
                }
            }
        }
        return null_mut::<u8>(); // Can't find any space
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        let mut previous: Option<*mut KChunkHeader> = None;
        for cptr in self.mut_iter() {
            let chunk: &mut KChunkHeader = &mut *cptr;
            if (*chunk).matches_layout(&layout) && (*chunk).user_data() == ptr {
                // this is it
                chunk.set_is_free(true);
                chunk.attempt_merge(self.heap_start, self.heap_end);
                if previous != None && (*previous.unwrap()).is_free() {
                    (*previous.unwrap()).attempt_merge(self.heap_start, self.heap_end);
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
        let previous: Option<*mut KChunkHeader> = None;
        let internal_align = max(layout.align(), size_of::<KChunkHeader>());
        let internal_size = Kalloc::adjust_size_with_align(new_size, internal_align);

        for cptr in self.mut_iter() {
            let chunk = &mut *cptr;
            if chunk.matches_layout(&layout) && (*chunk).user_data() == ptr {
                // this is it
                chunk.set_is_free(true);
                chunk.attempt_merge(self.heap_start, self.heap_end);
                if previous != None && (*previous.unwrap()).is_free() {
                    let p_chunk = &mut *previous.unwrap();
                    p_chunk.attempt_merge(self.heap_start, self.heap_end);
                    if p_chunk.size_with_padding() > internal_size {
                        // we can use prev: copy and be wary of
                        // overlap. we are writing forward from source
                        // to dest, and dest comes before source, so
                        // we should be fine
                        p_chunk.set_is_free(false);
                        p_chunk
                            .set_layout(Layout::from_size_align(new_size, layout.align()).unwrap());

                        let ptr_and_offset = Kalloc::adjust_ptr_with_align(
                            p_chunk.start_of_padding(),
                            internal_align,
                        );
                        p_chunk.set_alignment_offset(ptr_and_offset.1);

                        let dest = p_chunk.user_data();
                        let src = chunk.user_data();
                        for off in 0..chunk.user_size() {
                            dest.byte_offset(off as isize)
                                .write(src.byte_offset(off as isize).read());
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

                    // TODO this re-traverses part of the array. fix it.

                    chunk.set_is_free(false); // avoid clobbering side effects of future alloc optimizations

                    let dest =
                        self.alloc(Layout::from_size_align(new_size, layout.align()).unwrap());
                    if dest == null_mut::<u8>() {
                        panic!("Realloc Failure: Couldn't allocate a new chunk.");
                    }
                    let src = chunk.user_data();
                    for off in 0..chunk.user_size() {
                        dest.byte_offset(off as isize)
                            .write(src.byte_offset(off as isize).read());
                    }
                    chunk.set_is_free(true);
                    return dest;
                }
            }
        }
        panic!("Realloc Failure: Chunk not found. Have you changed your pointer or layout?")
    }
}
