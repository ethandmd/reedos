/// Allocator for use inside the kernel
///
/// This impliments GlobalAllocator but not Allocator
/// This is not a complete implimentation as it does not store layout information
/// This will always round up to a page on alloc. Don't be wasteful
/// 
/// Entrys are passed between rust functions as `usize`. This could be
/// refactored for readability into some kind of pointer by someone
/// who knows more about pointers in rust than me
///
/// However, this implimentation does not really conform to many of
/// the rust ideas about pointers and layouts, e.g. doesn't really
/// care about extents at the user visible level. This is a space for
/// improvement.

use core::alloc::*;
use core::arch::asm;

extern "C" {
    static _page_size: usize;
    
    static _heap_start: usize;
    static _heap_end: usize;
    static _heap_size: usize;
    
    static _entry_table_start: usize;
    static _entry_table_end: usize;
    static _entry_table_size: usize;
}

pub struct KernelAlloc {
    heap_start: usize,
    heap_end: usize,
    heap_size: usize,

    manage_start: usize,
    manage_end: usize,
    manage_size: usize,

}

#[global_allocator]
static ALLOCATOR: KernelAlloc = KernelAlloc {
    heap_start: _heap_start,
    heap_end: _heap_end,
    heap_size: _heap_size,

    manage_start: _entry_table_start,
    manage_end: _entry_table_end,
    manage_size: _entry_table_size,
};
/// Entry format is:
///
/// 64bit unsigned addr associated with the entry
/// 32bit flags
/// -> 0 for invalid
/// -> 1 for valid, not in use
/// -> 2 for valid, in use
/// 32bit unsigned size
/// -> size in (4k) pages 
/// 64bit unsigned addr of next next entry
///
/// This is a singly linked list of entries that should alternate
/// between in use and not in use
///
/// Entries are accessed only within asm blocks to avoid worrying
/// about rust arranging or otherwise messing with struct layout

impl KernelAlloc {
    // INTERNAL USE (to this impl block) ---------------------------------
    
    /// Gives addr of an invalid entry or None
    unsafe fn find_invalid(&self) -> Option<usize> {
	let mut out: usize;
	asm!(
	    "loop:",
	    "sleu {3}, {1}, {0}", // write 1 to {3} if {0} >= {1}
	    "bnez {3}, error",	  // overflow, start > end
	    "addi {3}, {0}, 8",	  // offset for flags
	    "lw {3}, {3}",	  // get flags
	    "beqz {3}, found",	  // check valid
	    "addi {0}, {0}, 24",  // offset to next entry
	    "j loop",		  // iterate
	    
	    "found:",
	    "mv {2}, {0}",	
	    "j ret",
	    
	    "error:",
	    "li {2} 0",
	    "j ret",
	    
	    "ret:",
	    in(reg) self.manage_start,
	    in(reg) self.manage_end,
	    out(reg) out,
	    out(reg) _
	);
	if out != 0 {
	    Some(out)
	} else {
	    None
	}
    }

    /// find a free chunk that is at least `size` pages, at return an
    /// addr to the entry, or None
    unsafe fn find_free_pages(&self, size: u32) -> Option<usize> {
	let mut ret: usize;
	asm!(
	    "loop:",
	    "sgeu {scratch1}, {end}, {start}", // ran off the end
	    "bnez {scratch1}, error",
	    "addi {scratch1}, {start}, 8",
	    "lw {scratch2}, {scratch1}", // get flags
	    "beq x0, {scratch2}, error", // invalid? shouldn't happen
	    "sub {scratch2}, {scratch2}, 2",
	    "beq x0, {scratch2}, next", // flags was 2, this one is is used already

	    "addi {scratch1}, {start}, 12", // offset for size
	    "lw {scratch2}, {scratch1}",    // load size
	    "bge {scratch2}, {size}, found", // we found it
	    
	    "next:",
	    "addi {scratch1}, {start}, 16", // offset for next ptr
	    "ld {start}, {scratch1}",	    // update pointer
	    "j loop",

	    "found:", 		// start is the entry that we want to return
	    "mov {ret}, {start}",
	    "j exit",

	    "error:",
	    "li {ret}, 0",
	    "j exit",

	    "exit:",
	    start = in(reg) self.manage_start,
	    end = in(reg) self.manage_end,
	    size = in(reg) size,
	    ret = out(reg) ret,
	    scratch1 = out(reg) _, 	// scratch reg
	    scratch2 = out(reg) _, 	// scratch reg
	);

	if ret == 0 {
	    None
	} else {
	    Some(ret)
	}
    }

    /// takes a pointer to a valid free entry, and merges it with the
    /// following valid free entry if that is possible
    unsafe fn merge_forward(&mut self, entry: usize) {
	asm!(
	    "addi {scratch1}, {current}, 16",		// get offset of next link
	    "ld {scratch2}, {scratch1}",		// addr of next entry in scratch2
	    "beq {scratch2}, {end}, ret",		// is current the last entry? then exit
	    
	    "addi {scratch1}, {scratch2}, 8",		// next entry flag offset
	    "lw {scratch1}, {scratch1}",		// load flags
	    "beq {scratch1}, x0, exit",			// was invalid ? get out of here
	    
	    "subi {scratch1}, {scratch2}, 2",
	    "beq {scratch1}, x0, exit",			// was in use, get out

							// next is also free and valid
	    "addi {scratch1}, {scratch2}, 12",		// next entry size offset
	    "lw {scratch3}, {scratch1}",		// load next size in scratch3
	    
	    "addi {scratch1}, {current}, 12",		// current entry size offset
	    "lw {scratch1}, {scratch1}",		// current size in scratch1
	    "add {scratch3}, {scratch3}, {scratch1}",	// expand first to include both (in scratch3)
	    
	    "addi {scratch1}, {current}, 12",		// current entry size offset
	    "sw {scratch1}, {scratch3}",		// write current size back
	    
	    "addi {scratch1}, {scratch2}, 8",		// next entry flag offset
	    "sw, {scratch1}, x0",		        // mark next invalid
	    
	    "addi {scratch1}, {scratch2}, 16",		// next entry next pointer offset
	    "ld {scratch1}, {scratch1}",		// get next.next pointer
	    "addi {scratch3}, {current}, 16",		// current entry next pointer offset
	    "sd {scratch3}, {scratch1}",		// write current entry next pointer
	    
	    "exit:",
	    end = in(reg) self.manage_end,
	    current = in(reg) entry,
	    scratch1 = out(reg) _,
	    scratch2 = out(reg) _,
	    scratch3 = out(reg) _
	);
    }

    /// gets a new entry, and sets to to be
    /// free with the given size, possibly merging it with the next
    /// free entry, then returns it
    unsafe fn init_free(&mut self, next: usize, size: u32) -> usize {
	let mut unused:usize = self.find_invalid().expect("Could not find a unused heap entry");
	asm!(
	    "addi {scratch1}, {entry}, 8", // flag offset
	    "li {scratch2}, 1",		   // valid, not in use
	    "sw {scratch1}, {scratch2}",   // write
	    "addi {scratch1}, {entry}, 12", // size offset
	    "sw {scratch1}, {size}",	    // write size
	    "addi {scratch1}, {entry}, 16", // next offset
	    "sd {scratch1}, {next}",	    // write next 

	    entry = in(reg) unused,
	    size = in(reg) size,
	    next = in(reg) next,
	    scratch1 = out(reg) _,
	    scratch2 = out(reg) _
	);
	self.merge_forward(unused);
	unused
    }

    // EXTERNAL USE ------------------------------------------------------
    
    /// find and size a chunk that is at least `size` pages, and edits
    /// the entry list to mark said entry as used, and slice off the
    /// free space, possibly merging it with surrounding free space
    ///
    /// returns the pointer to the associated data
    unsafe fn find_and_edit_entry(&mut self, size: u32) -> usize {
	let entry: usize = self.find_free_pages(size).expect("Could not find a free heap entry with the right size");
	let mut ret: usize;
	let mut slice_size: u32;
	let mut next_entry: usize;
	asm!(
	    "addi {scratch1}, {entry}, 8",  // flag offset
	    "li {scratch2}, 2",		    // valid + in use
	    "sw {scratch1}, {scratch2}",    // write flags

	    "addi {scratch1}, {entry}, 16", // next offset
	    "ld {next_entry}, {scratch1}",	    // load next
	    
	    "addi {scratch1}, {entry}, 12", // size offset
	    "lw {scratch2}, {scratch1}",	    // load size
	    "sub {slice_size}, {scratch2}, {size}",  // slice = current - requested

	    "ld {ret}, {entry}",	   // get data ptr
	    entry = in(reg) entry,
	    size = in(reg) size,
	    slice_size = out(reg) slice_size,
	    next_entry = out(reg) next_entry,
	    ret = out(reg) ret,
	    scratch1 = out(reg) _,
	    scratch2 = out(reg) _
	);
	if slice_size != 0 {
	    // we need to slice
	    let slice: usize = self.init_free(next_entry, slice_size);
	    let addr_inc: usize = _page_size * (size as usize);
	    asm!(
		"addi {scratch1}, {entry}, 16", // next link offset
		"sd {scratch1}, {slice}",	// write next link
		"ld {scratch1}, {entry}",	// load addr
		"add {scratch1}, {scratch1}, {addr_inc}", // adjust data ptr
		"sd {entry}, {scratch1}",		  // write data ptr
		entry = in(reg) entry,
		slice = in(reg) slice,
		addr_inc = in(reg) addr_inc,
		scratch1 = out(reg) _
	    );
	} else {
	    // no need to slice, exact match, no action necessary
	}
	ret
    }

    /// frees a valid allocated pointer, panics if it can't find
    /// it. Merges if necessary
    unsafe fn free_entry(&mut self, data_ptr: usize) {
	let mut ret: usize;
	asm!(
	    "loop:",
	    "bgeu {current}, {end_addr}, error", // catch runoff
	    "ld {scratch1}, {current}",		 // get data ptr
	    "beq {scratch1}, {data_ptr}, found", // is this entry the one?
	    "addi {current}, {current}, 16",	 // next offset
	    "ld {current}, {current}",		 // get next entry
	    "j loop",				 // iterate

	    "error:",
	    "li {ret}, 0",
	    "j exit",

	    "found:",
	    "mv {ret}, {current}",
	    "addi {current}, {current}, 8", // flag offset
	    "li {scratch1}, 1",		    // valid + not in use
	    "lw {current}, {scratch1}",	    // write flags
	    "exit:",
	    current = inout(reg) self.manage_start,
	    end_addr = in(reg) self.manage_end,
	    data_ptr = in(reg) data_ptr,
	    scratch1 = out(reg) _,
	    ret = out(reg) ret
	);

	if ret == 0 {
	    panic!("Freed something that wasn't allocated");
	} else {
	    self.merge_forward(ret);	
	}
    }
}

unsafe impl Sync for KernelAlloc {}


unsafe impl GlobalAlloc for KernelAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
	let size: usize = layout.size(); // how many bytes
	let align: usize = layout.align(); // how many bytes aligned this should be, 2^{num zeros} of addr roughly

	if align > _page_size {
	    panic!("Can't allocate things aligned larger than a page");
	} else if size/ _page_size > u32::MAX as usize {
	    panic!("Alloc request too big. That seems unlikely, you should check your allocs");
	} else {
	    self.find_and_edit_entry((size / _page_size) as u32) as *mut u8
	}
    }
    
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
	// I don't store layout info atm, so I will just check ptrs
	// TODO add checks to be fully compliant with the standard

	let internal_addr: usize = ptr as usize;
	self.free_entry(internal_addr);
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
	let ret: *mut u8 = self.alloc(layout);

	ret.write_bytes(0, layout.size());
	ret
    }
    
    unsafe fn realloc(
        &self,
        ptr: *mut u8,
        layout: Layout,
        new_size: usize
    ) -> *mut u8 {
	// ignoring layout for now, becuase I don't store that on alloc

	let out: *mut u8 = self.alloc(
	    core::alloc::Layout::from_size_align(new_size,
						 layout.align()).unwrap());
	for offset in 0..layout.size() {
	    out.offset(offset as isize).write(
		ptr.offset(offset as isize).read()
	    );
	}
	self.dealloc(ptr, layout);
	out
    }
}
