/// General allocation, basically malloc/kalloc type thing
use crate::hw::param::*;
use core::assert;
use crate::vm::palloc::PagePool;

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
type Header = [u64; param::PAGE_SIZE/64];

pub struct GAlloc {
    root: *mut Header,
}

// gives the index of the lowest set bit or None
// fn lowest_set_bit(field: u64) -> Option<usize> {
//     let mut i = 0;
//     while (i < 64 &&
//            !((field >> i) & 0x1)) {
//         i += 1;
//     }
//     match i {
//         64 => {
//             None
//         },
//         _ => {
//             i
//         }
//     }
// }

//same but for highest
// fn highest_set_bit(field: u64) -> Option<usize> {
//     let mut i = 63;
//     while (i >= 0 &&
//            !((field >> i) & 0x1)) {
//         i -= 1;
//     }
//     match i {
//         0 => {
//             None
//         },
//         _ => {
//             i
//         }
//     }
// }

// not efficient. make a lower bit mask with said # of ones
fn make_mask(mut num_ones: usize) -> u64 {
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
    let mut i = 1;
    while (i < 64) {
        s |= s >> i;
    }
    s + 1
}

fn get_page() -> *mut u8 {
    unsafe { (*crate::vm::PAGEPOOL).palloc() }
}

impl GAlloc {
    pub fn new(page_allocator: PagePool) -> Self {
        let page =  as *mut Header;
        page.0 = 1;
        page.1 = 0;
        // level 1 page with no valid pages
        GAlloc {
            root: page
        }
    }

    // readability helpers
    fn set_level(mut self, level: u64) {
        self.root.0 = level;
    }

    fn set_valid_bits(mut self, bit_mask: u64) {
        self.root.1 |= bit_mask;
    }

    // clears the bits specified as arg
    fn clear_valid_bits(mut self, bit_mask: u64) {
        self.root.1 &= !bit_mask;
    }

    fn level(self) -> u64 {
        self.root.0
    }

    fn valid(self) -> u64 {
        self.root.1
    }

    //TODO drop? What does that even mean here

    fn search_data_layer(mut size: u64, dl_mask: u64) -> Option<u64> {
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

    fn walk_alloc(size: usize, root: *mut Header) -> Option<*mut usize> {
        if root.level() != 1 {
            for i in (2..64).step_by(2) {
                match walk_alloc(size, *(root.i)) {
                    None => {},
                    ret => { return ret; }
                }
            }
            return None;
        } else {
            let open: isize = -1; // neg if full, 0-63 for first empty
            for i in (2..64).step_by(2) {
                if (root.valid() >> i) & 0x1 == 0 {
                    if open == -1 { open = i; }
                    continue;
                }

                match search_data_layer(size, root[i+1]) {
                    None => {},
                    Some(idx) => {
                        // found one, make pointer
                        let in_use = round_up(size) / 8; // how many to mark in use
                        root[i+1] = root[i+1] | (make_mask(in_use) << idx);
                        return Some(root[i].offset(idx));
                    }
                }
            }
            // couldn't find anything, try to add another indirect layer
            if open >= 0 {
                let mut page = palloc() as *mut Header;
                root[open] = page;
                page[0] = root[0] - 1;
                page[1] = 0;    // entirely empty;
                root[1] = root[1] | (1 << open); // down link valid
                root[1] = root[1] & !(1 << (open+1)); // mask no longer valid
                return walk_alloc(size, root[open]);
            }
            return None;
        }
    }

    pub fn alloc(mut self, size: usize) -> Option<*mut usize> {
        assert!(size <= PAGE_SIZE, "GAlloc is only sub-page size");
        match walk_alloc(size, self.root) {
            Some(ret) => { Some(ret) },
            None => {
                let new_root = palloc() as *mut Header;
                new_root[0] = self.root[0] + 1;
                new_root[1] = 0x4; // single valid entry, old root
                new_root[2] = self.root;
                self.root = new_root;
                walk_alloc(size, self.root)
            }
        }
    }

    fn walk_dealloc(ptr: *mut usize, size: usize, root: *mut Header) {
        let test_ptr = ptr as usize & !(PAGE_SIZE - 1);
        if Header[0] != 1 {
            for i in (2..64).step_by(2) {
                if root[1] >> i == 0 {continue;}
                walk_dealloc(ptr, size, root[i]);
            }
        } else {
            // bottom level, search for match
            for i in (2..64).step_by(2) {
                if root[1] >> i == 0 {continue;}
                if root[i] as usize == test_ptr {
                    // match!
                    let offset = ptr as usize & (PAGE_SIZE - 1);
                    let clear_mask = make_mask(round_up(size) / 8);
                    root[i+1] = root[i+1] & !(clear_mask << offset);
                }
            }
        }
    }

    pub fn dealloc(mut self, ptr: *mut usize, size: usize) {
        walk_dealloc(ptr, size, self.root);
    }
}
