use core::mem::size_of;

use crate::hw::param::PAGE_SIZE;
use super::{palloc::Page, VmError};

const MAX_CHUNK_SIZE: usize = 4080; // PAGE_SIZE - ZONE_HEADER_SIZE - HEADER_SIZE = 4096 - 8 = 4088.
const HEADER_SIZE: usize = size_of::<Header>();
const ZONE_SIZE: usize = 8;
const HEADER_USED: usize = 1 << 12; // Chunk is in use flag.

// 8 byte minimum allocation size,
// 4096-8-8=4080 byte maximum allocation size.
// Guarantee that address of header + header_size = start of data.
// Size must be <= 4080 Bytes.
// Bits 0-11 are size (2^0 - (2^12 - 1))
// Bit 12 is Used.
//
// Header:
// ┌────────────────────────────────────┬─┬──────────────┐
// │    Unused / Reserved               │U│ Chunk Size   │
// └────────────────────────────────────┴─┴──────────────┘
// 63                                   12 11            0
//
#[repr(C)]
struct Header {
    fields: usize, // Could be a union?
}

// An allocation zone is the internal representation of a page.
// Each zone contains the address of the next zone (page aligned),
// plus the number of in use chunks within the zone (refs count).
//
// Zone.next:
// ┌──────────────────────────────────────┬──────────────┐
// │  next zone address (page aligned)    │ refs count   │
// └──────────────────────────────────────┴──────────────┘
// 63                                     11             0
//
#[repr(C)]
struct Zone {
    base: *mut usize,   // This zone's address.
    next: usize,        // Next zone's address + this zone's ref count.
}

struct Kalloc {
    head: *mut usize, // Address of first zone.
    end: *mut usize,
}

enum KallocError {
    MaxRefs,
    MinRefs,
    NullZone,
}

impl From<*mut usize> for Header {
    fn from(src: *mut usize) -> Self {
        let fields = unsafe { src.read() };
        Header { fields }
    }
}

impl Header {
    fn new(size: usize) -> Self {
        assert!(size <= MAX_CHUNK_SIZE);
        Header { fields: size }
    }

    fn chunk_size(&self) -> usize {
        self.fields & 0xFFF
    }

    fn is_free(&self) -> bool {
        self.fields & !HEADER_USED == 0
    }

    fn set_used(&mut self) {
        self.fields = self.fields | HEADER_USED;
    }

    fn set_unused(&mut self) {
        self.fields = self.fields & !HEADER_USED;
    }

    // Clear size bits. Set size bits to size.
    fn set_size(&mut self, size: usize) {
        self.fields = (self.fields & !(0x1000 - 1)) | size;
    }

    // Unsafe write header data to memory at dest.
    fn write_to(&self, dest: *mut usize) {
        unsafe {
            dest.write_volatile(self.fields);
        }
    }

    // Takes an existing chunk and splits it into a chunk of 'new_size' + the remainder.
    fn split(&mut self, new_size: usize, cur_addr: *mut usize) -> (Header, *mut usize) {
        let old_size = self.chunk_size();
        let next_size = old_size - new_size;
        self.set_size(new_size);
        let next_addr = cur_addr.map_addr(|addr| addr + HEADER_SIZE + new_size);
        let next_header = Header { fields: next_size - HEADER_SIZE }; // make space for inserted header
        next_header.write_to(next_addr);
        (next_header, next_addr)
    }
}

// Assumes the first byte of a zone is the zone header.
// Next byte is the chunk header.
impl From<*mut usize> for Zone {
    fn from(src: *mut usize) -> Self {
        Zone { 
            base: src, 
            next: unsafe { src.read() }
        }
    }
}

impl Zone {
    fn new(base: *mut usize) -> Self {
        Zone {
            base,
            next: 0x0,
        }
    }

    fn get_refs(&self) -> usize {
        self.next & (4095)
    }
    
    fn get_next(&self) -> Result<usize, KallocError> {
        let next_addr = self.next & !(PAGE_SIZE - 1);
        if next_addr == 0x0 {
            Err(KallocError::NullZone)
        } else {
            Ok(next_addr)
        }
    }
    
    // Read the next field to get the next zone address.
    // Discard this zone's refs count.
    // Write base address with next zone address and new refs count.
    #[inline(always)]
    unsafe fn write_refs(&mut self, new_count: usize) {
        let next_addr = match self.get_next() {
            Err(_) => 0x0,
            Ok(ptr) => ptr,
        };

        self.base.write(next_addr | new_count);
    }

    // Read the current next field to get the refs count.
    // Discard this zone's next addr.
    // Write base address with new next zone address and refs count.
    unsafe fn write_next(&mut self, new_next: *mut usize) {
        let refs = self.get_refs();
        self.base.write(new_next.addr() | refs);
    }

    fn increment_refs(&mut self) -> Result<(), KallocError> {
        let new_count = self.get_refs() + 1;
        if new_count > 510 { 
            Err(KallocError::MaxRefs) 
        } else {
            unsafe { self.write_refs(new_count); }
            Ok(())
        }
    }

    fn decrement_refs(&mut self) -> Result<(), KallocError> {
        // Given a usize can't be < 0, I want to catch that and not cause a panic.
        // This may truly be unnecessary, but just want to be cautious.
        let new_count = self.get_refs() - 1;
        if (new_count as isize) < 0 {
            Err(KallocError::MinRefs)
        } else {
            unsafe { self.write_refs(new_count); }
            Ok(())
        }
    }

    fn next_zone(&self) -> Result<Zone, KallocError> {
        let next_addr = self.get_next()?;
        Ok(Zone::from(next_addr as *mut usize))
    }
}

unsafe fn write_zone_header_pair(zone: Zone, header: Header) {
    let base = zone.base;
    base.write(zone.next);
    base.byte_add(1).write(header.fields);
}

impl Kalloc {
    fn new(start: Page) -> Self {
        // Make sure start of allocation pool is page aligned.
        assert_eq!(start.addr.addr() & (PAGE_SIZE - 1), 0);
        // New page is the first zone in the Kalloc pool.
        let zone = Zone::new(start.addr);
        let head = Header::new(MAX_CHUNK_SIZE);
        unsafe { write_zone_header_pair(zone, head); }
        Kalloc {
            head: start.addr,
            end: start.addr.map_addr(|addr| addr + 0x1000),
        }
    }

    fn alloc(&mut self, mut size: usize) -> Result<*mut usize, VmError> {
        // Start tracks address of each header.
        let mut start = self.head;
        let mut head = Header::from(start);
        size = if size < 8 {8} else {size};

        // Remove redundancy + use some helper fns.
        while start != self.end {
            let chunk_size = head.chunk_size();
            if chunk_size < size || !head.is_free() {
                start = start.map_addr(|addr| addr + HEADER_SIZE + chunk_size);
                head = Header::from(start);
            } else {
                head.set_used();
                if size != chunk_size {
                    let (next, next_addr) = head.split(size, start);
                    next.write_to(next_addr);
                }
                return Ok(start.map_addr(|addr| addr + HEADER_SIZE))
            }
        }
        Err(VmError::Koom)
    }

    // TODO if you call alloc in order and then free in order this
    // doesn't merge, as you can't merge backwards. Consider a merging
    // pass when allocting.
    fn free(&mut self, ptr: *mut usize) {
        let chunk_loc = ptr.map_addr(|addr| addr - HEADER_SIZE);
        let mut head = Header::from(chunk_loc);
        assert!(!head.is_free(), "Kalloc double free.");
        head.set_unused();
        let next = Header::from(chunk_loc.map_addr(
            |addr| addr + HEADER_SIZE + head.chunk_size()));
        if !(next.is_free()) {
            // back to back free, merge
            head.set_size(head.chunk_size() + HEADER_SIZE + next.chunk_size())
        }
        head.write_to(chunk_loc);
    }
}
