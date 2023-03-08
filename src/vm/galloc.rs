/// General allocation, basically malloc/kalloc type thing
use crate::hw::param::*;
use core::assert;
use crate::vm::*;

/// This is either a data layer or an indirect layer
///
/// If it is indirect, then the first u64 is the level. Level n points
/// to n-1, and 1 points to data layers
///
/// If it is indirect, the second u64 is the valid/in use bits of the
/// corresponding u64s in the current header.
///
/// If this is an indirect header, then all futher u64 are paried. The
/// even indexed (first) u64 is a pointer dwon one level. The odd
/// (second) one is the valid mask for that link. If the link is to a
/// data layer, then it corresponds to the parts of the data layer in
/// use. If the link is to another indirect layer, then ignore this
/// and decend and check the second u64 of that layer instead. (In
/// fact it should be marked invalid.)
///
/// If this is a data layer, then the entire page is naturally aligned
/// data. By that I mean that a pow of 2 chunk of size n is n-byte
/// aligned.

// I'd use page size but rust won't let me
// type Header = [u64; 4096/64];

#[repr(C)]
#[derive(Clone, Copy)]
struct HeaderPair {
    valid: u64,
    down: *mut Header,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct Indirect {
    level: u64,
    valid: u64,
    contents: [HeaderPair; 255],
}

#[derive(Clone, Copy)]
union Header {
    data: [u64; 4096/64],
    indirect: Indirect,
}

pub struct GAlloc {
    root: *mut Header,
}

// not efficient. make a lower bit mask with said # of ones
fn make_mask(mut num_ones: u64) -> u64 {
    let mut out = 0;
    while num_ones > 0 {
        out = (out << 1) | 1;
        num_ones -= 1;
    }
    out
}

// pow of two that fits s
/* stolen: https://graphics.stanford.edu/~seander/bithacks.html#RoundUpPowerOf2 */
fn round_up(mut s:u64) -> u64 {
    s -= 1;
    for i in 1..64 {
        s |= s >> i;
    }
    s + 1
}

fn get_page() -> Result<*mut usize, VmError> {
    match unsafe { PAGEPOOL.get_mut().unwrap().palloc() } {
        Err(e) => {
            Err(e)
        },
        Ok(page) => {
            Ok(page.addr as *mut usize)
        }
    }
}


impl Drop for GAlloc {
    fn drop(&mut self) {
        panic!("Dropped your general allocator")
    }
}

impl GAlloc {
    pub fn new() -> Self {
        let page = match get_page() {
            Err(e) => {
                panic!("Couldn't initialize the header for general alloc: {:?}", e)
            },
            Ok(addr) => {
                addr as *mut Header
            }
        };
        unsafe {
            (*page).indirect.level = 1;
            (*page).indirect.valid = 0;
        }
        // level 1 page with no valid pages
        GAlloc {
            root: page
        }
    }

    fn search_data_layer(size: u64, dl_mask: u64) -> Option<u64> {
        let size = round_up(size) / 8; // pow 2, in usize units
        let search_mask = make_mask(size);

        let mut i = 0;
        while i < 64 {
            if (dl_mask >> i) & search_mask == 0 {
                // clear bits
                return Some(i);
            } else {
                i += size;      // skip size places
            }
        }
        None
    }

    unsafe fn walk_alloc(size: usize, root: &mut Header) -> Result<*mut usize, VmError> {
        let mut open: isize = -1; // neg if full, 0-31 for first empty
        if root.indirect.level != 1 {
            for i in 0..32 {
                if (root.indirect.valid >> i) & 0x1 == 0{
                    // invalid down link
                    if open != -1 { open = i; }
                } else {
                    // this is a down link we can follow
                    let down = &mut *root.indirect.contents[i as usize].down;
                    match Self::walk_alloc(size, down) {
                        Err(_) => {},
                        ret => { return ret; }
                    }
                }
            }
            // checked all valid down links and none of them are valid
            // now check if we can add a valid one (was there a hole)

            if open != -1 {
                let page: *mut Header = match get_page() {
                    Err(e) => {
                        return Err(e);
                    },
                    Ok(addr) => {
                        addr as *mut Header
                    }
                };
                // insert a new page
                let p_ref = &mut *page;
                p_ref.indirect.level = root.indirect.level -1;
                p_ref.indirect.valid = 0;
                root.indirect.contents[open as usize].down = p_ref;
                // root.indirect.contents[open as usize].valid is not needed, as p_ref is not a data layer
                root.indirect.valid = root.indirect.valid | 1 << open;
                return Self::walk_alloc(size, p_ref);
            }
            // no space and no holes for further intermediate levels
            // in any case, pass the error up
            return Err(VmError::GNoSpace);
        } else {
            // this is a level 1 layer, and points to data layers
            for i in 0..32 {
                let i = i as usize;
                if (root.indirect.valid >> i) & 0x1 == 0 {
                    // this is a data page down link that isn't in use
                    if open == -1 { open = i as isize; }
                    continue;
                }

                match Self::search_data_layer(size as u64,
                                              root.indirect.contents[i].valid) {
                    None => {},
                    Some(idx) => {
                        // found space, mark and make pointer
                        let in_use = round_up(size as u64) / 8; // how many to mark in use
                        root.indirect.contents[i].valid =
                            root.indirect.contents[i].valid | (make_mask(in_use) << idx);
                        let data_page = root.indirect.contents[i].down as *mut usize;
<<<<<<< HEAD
                        return Ok(data_page.offset(idx as isize));
=======
                        return Ok(unsafe { data_page.offset(idx as isize) });
>>>>>>> origin/6-Allocator
                    }
                }
            }
            // couldn't find anything, try to add another data page
            if open == -1 {
                let open = open as usize;
                let page: *mut Header = match get_page() {
                    Err(e) => {
                        return Err(e);
                    },
                    Ok(addr) => {
                        addr as *mut Header
                    }
                };
                root.indirect.contents[open].down = page;
                root.indirect.contents[open].valid = 0; // all free
                // don't set page meta, because this is a data page
                root.indirect.valid = root.indirect.valid | (1 << open); // down link valid
                return Self::walk_alloc(size, &mut *(root.indirect.contents[open].down));
            }
            return Err(VmError::GNoSpace);
        }
    }

    pub fn alloc(&mut self, size: usize) -> Result<*mut usize, VmError> {
        assert!(size <= PAGE_SIZE, "GAlloc is only sub-page size");
        match unsafe {Self::walk_alloc(size, &mut (*self.root)) } {
            Ok(ret) => { Ok(ret) },
            Err(_) => {
                // alloc failed. try to bump the root up (note that
                // this may also fail if the issue was out of pages)
                let mut page: *mut Header = match get_page() {
                    Err(e) => {
                        return Err(e);
                    },
                    Ok(addr) => {
                        addr as *mut Header
                    }
                };
                unsafe {
                    (*page).indirect.level = (*self.root).indirect.level + 1; // bump level
                    (*page).indirect.valid = 1; // single valid page (old root)
                    (*page).indirect.contents[0] = HeaderPair {
                        valid: 0, // unused since root is not a data page
                        down: self.root,
                    };
                }
                self.root = page;
                match unsafe { Self::walk_alloc(size, &mut (*self.root)) } {
                    Err(e) => {
                        Err(e)
                    },
                    Ok(addr) => {
                        Ok(addr)
                    }
                }
            }
        }
    }

    // returns (did_we_find_it, should_we_keep_this_branch)
    unsafe fn walk_dealloc(ptr: *mut usize, size: usize, root: &mut Header) -> (bool, bool) {
        let test_ptr = ptr as usize & !(PAGE_SIZE - 1); // should match data_page base pointer
        if root.indirect.level != 1 {
            // down links are not data pages
            let valid = root.indirect.valid;
            if valid == 0 {
                return (false, false);
            }
            let mut should_we_keep = false;
            for i in 0..32 {
                if (valid >> i) & 1 == 0 {continue;}
                match Self::walk_dealloc(ptr, size, &mut (*root.indirect.contents[i].down)) {
                    (true, true) => {
                        return (true, true);
                    },
                    (false, true) => {
                       // keep searching
                        should_we_keep = true;
                    },
                    (found, false) => {
                        // trim branch and maybe report findings
                        root.indirect.valid = root.indirect.valid & !(1 << i);
                        // TODO free the said down link
                        if root.indirect.valid == 0 {
                            // nothing more to check, report findings
                            return (found, false);
                        } else if found {
                            return (true, true);
                        }
                    }
                }
            }
            if should_we_keep {
                return (false, true);
            } else {
                return (false, false);
            }
        } else {
            // downlinks are data pages, search for match
            let valid = root.indirect.valid;
            for i in 0..32 {
                if (valid >> i) & 1 == 0 {continue;}
                if root.indirect.contents[i].down as usize == test_ptr {
                    // match!
                    let offset = ptr as usize & (PAGE_SIZE - 1);
                    let clear_mask = make_mask(round_up(size as u64) / 8);
                    root.indirect.contents[i].valid =
                        root.indirect.contents[i].valid & !(clear_mask << offset);
                    if root.indirect.contents[i].valid == 0 {
                        // free data page
                        // TODO free page
                        root.indirect.valid = valid & !(1 << i);
                        if root.indirect.valid == 0 {
                            // cleanup this indirect layer
                            return (true, false);
                        } else {
                            return (true, true);
                        }
                    } else {
                        return (true, true);
                    }
                }
            }
            if valid == 0 {
                return (false, false);
            } else {
                return (false, true);
            }
        }
    }

    pub fn dealloc(&mut self, ptr: *mut usize, size: usize) {
        unsafe {
            // TODO consider mechanism for undoing root bump / when to do that
            Self::walk_dealloc(ptr, size, &mut (*self.root));
        }
    }
}
