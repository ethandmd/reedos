//! Kernel Virtual Memory Allocator.
use core::mem::size_of;

use super::{palloc, palloc::Page, pfree, VmError};
use crate::hw::param::PAGE_SIZE;

pub const MAX_CHUNK_SIZE: usize = 4080; // PAGE_SIZE - ZONE_HEADER_SIZE - HEADER_SIZE = 4096 - 8 - 8 = 4080.
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
// Header.fields:
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
#[derive(Copy, Clone)]
struct Zone {
    base: *mut usize, // This zone's address.
    next: usize,      // Next zone's address + this zone's ref count.
}

/// Kernel Virtual Memory Allocator.
/// Kalloc is comprised of `Zones` (physical pages). Each
/// zone is broken up into smaller chunks as memory is allocated
/// and merged into larger chunks as memory is deallocated.
/// Each allocation, `x` , must satisfy `0<= x <= 4080` bytes.
/// All allocations will be automatically rounded up to be
/// 8 byte aligned.
///
/// A generic zone with the first zone containing 1 in use chunk and
/// a second full zone with one in use chunk might look like:
/// ```text
///           Kalloc {
///
///    ┌────────start
///    │
/// ┌──┼────────end
/// │  │
/// │  │      }
/// │  │
/// │  │    ┌──────────────────────────────────────┬──────────────┐  (zone header)
/// │  └─┬──┤►           0x80089e000               │  0x1         │   0x80089d000
/// │    │  └──────────────────────────────────────┴──────────────┘
/// │    │  63                                     11             0
/// │    │  ┌────────────────────────────────────┬─┬──────────────┐  (chunk header)
/// │    │  │            Unused / Reserved       │1│  0x008       │   0x80089d008
/// │    │  └────────────────────────────────────┴─┴──────────────┘
/// │    │  63                                  12 11             0
/// │    │  ┌─────────────────────────────────────────────────────┐     (data)
/// │    │  │            0x8BADF00D                               │   0x80089d010
/// │    │  └─────────────────────────────────────────────────────┘
/// │    │  63                                                    0
/// │    │  ┌────────────────────────────────────┬─┬──────────────┐  (chunk header)
/// │    │  │            Unused / Reserved       │0│  0xfe0       │   0x80089d018
/// │    │  └────────────────────────────────────┴─┴──────────────┘
/// │    │  63                                  12 11             0
/// │    │
/// │    │
/// │    │                             ...
/// │    │
/// │    │  ┌──────────────────────────────────────┬──────────────┐  (zone header)
/// │    └──►           0x0                        │  0x1         │   0x80089e000
/// │       └──────────────────────────────────────┴──────────────┘
/// │       63                                     11             0
/// │       ┌────────────────────────────────────┬─┬──────────────┐  (chunk header)
/// │       │           Unused / Reserved        │1│  0xff0       │   0x80089e008
/// │       └────────────────────────────────────┴─┴──────────────┘
/// │       63                                  12 11             0
/// │       ┌─────────────────────────────────────────────────────┐     (data)
/// │       │                         0x0                         │   0x80089e010
/// │       └                                                     ┘
/// │       63                         │                          0
/// │                                  │
/// │                                  │ [usize; 510]
/// │                                  │
/// │                                  │
/// │                                  │
/// │                                  ▼
/// │       ┌                                                     │     (data)
/// │       │                        0x1fd                        │   0x80089eff8
/// │       └─────────────────────────────────────────────────────┘
/// │       63                                                    0
/// │                                                                (end of pool)
/// └───────────────────────────────────────────────────────────────► 0x80089d000
///```
pub struct Kalloc {
    head: *mut usize, // Address of first zone.
    end: *mut usize,
}

#[derive(Debug)]
pub enum KallocError {
    MaxRefs,
    MinRefs,
    NullZone,
    OOM,
    Void,
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
        self.fields & HEADER_USED == 0
    }

    fn set_used(&mut self) {
        self.fields |= HEADER_USED;
    }

    fn set_unused(&mut self) {
        self.fields &= !HEADER_USED;
    }

    // Clear size bits. Set size bits to size.
    fn set_size(&mut self, size: usize) {
        self.fields = (self.fields & !(0x1000 - 1)) | size;
    }

    // Unsafe write header data to memory at dest.
    fn write_to(&self, dest: *mut usize) {
        unsafe {
            dest.write(self.fields);
        }
    }

    // Takes an existing chunk and splits it into a chunk of 'new_size' + the remainder.
    fn split(&mut self, new_size: usize, cur_addr: *mut usize) -> (Header, *mut usize) {
        let old_size = self.chunk_size();
        let next_size = old_size - new_size;
        self.set_size(new_size);
        self.write_to(cur_addr);
        let next_addr = cur_addr.map_addr(|addr| addr + HEADER_SIZE + new_size);
        let next_header = Header {
            fields: next_size - HEADER_SIZE,
        }; // make space for inserted header
        next_header.write_to(next_addr);
        (next_header, next_addr)
    }

    fn merge(&mut self, next: Self, next_addr: *mut usize) {
        assert!(next.is_free());
        assert!(self.is_free());
        let size = self.chunk_size() + HEADER_SIZE + next.chunk_size();
        self.set_size(size);
        //self.write_to(addr);
        unsafe {
            next_addr.write(0);
        }
    }
}

// Assumes the first usize of a zone is the zone header.
// Next usize is the chunk header.
impl From<*mut usize> for Zone {
    fn from(src: *mut usize) -> Self {
        Zone {
            base: src,
            next: unsafe { src.read() },
        }
    }
}

impl Zone {
    fn new(base: *mut usize) -> Self {
        Zone { base, next: 0x0 }
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
        let next_addr = self.get_next().unwrap_or(0x0);
        self.next = next_addr | new_count;
        self.base.write(self.next);
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
            unsafe {
                self.write_refs(new_count);
            }
            Ok(())
        }
    }

    fn decrement_refs(&mut self) -> Result<usize, KallocError> {
        // Given a usize can't be < 0, I want to catch that and not cause a panic.
        // This may truly be unnecessary, but just want to be cautious.
        let new_count: isize = (self.get_refs()) as isize - 1;
        if (new_count as isize) < 0 {
            Err(KallocError::MinRefs)
        } else {
            unsafe {
                self.write_refs(new_count as usize);
            }
            Ok(new_count as usize)
        }
    }

    fn next_zone(&self) -> Option<Zone> {
        if let Ok(addr) = self.get_next() {
            Some(Zone::from(addr as *mut usize))
        } else {
            None
        }
    }

    // Only call from Kalloc.shrink_pool() to ensure this is not the first
    // zone in the pool.
    fn free_self(&mut self, mut prev_zone: Zone) {
        assert!(self.get_refs() == 0);
        // todo!("Relies on sequential page allocation.");
        // let prev_base = unsafe { self.base.byte_sub(0x1000) };
        // let mut prev_zone = Zone::from(prev_base);
        // // ^ BUG: not guaranteed sequential
        if let Some(next_zone) = self.next_zone() {
            unsafe {
                prev_zone.write_next(next_zone.base);
            }
        } else {
            unsafe {
                prev_zone.write_next(core::ptr::null_mut::<usize>());
            }
        }
        let _ = pfree(Page::from(self.base));
    }

    // Scan this zone for the first free chunk of size >= requested size.
    // First 8 bytes of a zone is the Zone.next field.
    // Second 8 bytes is the first header of the zone.
    fn scan(&mut self, size: usize) -> Option<*mut usize> {
        // Start and end (start + PAGE_SIZE) bounds of zone.
        let (mut curr, end) = unsafe { (self.base.add(1), self.base.add(PAGE_SIZE / 8)) };
        // Get the first header in the zone.
        let mut head = Header::from(curr);

        while curr < end {
            let chunk_size = head.chunk_size();
            if chunk_size < size || !head.is_free() {
                let (mut prev, trail) = (head, curr);
                curr = curr.map_addr(|addr| addr + HEADER_SIZE + chunk_size);
                head = Header::from(curr);

                // TODO: Is not pretty, make pretty.
                if prev.is_free() && head.is_free() {
                    prev.merge(head, curr);
                    prev.write_to(trail);
                    (head, curr) = (prev, trail);
                }
            } else {
                alloc_chunk(size, curr, self, &mut head);
                return Some(curr.map_addr(|addr| addr + HEADER_SIZE));
            }
        }
        None
    }
}

fn alloc_chunk(size: usize, ptr: *mut usize, zone: &mut Zone, head: &mut Header) {
    zone.increment_refs()
        .expect("Maximum zone allocation limit exceeded.");
    head.set_used();
    head.write_to(ptr);

    if size != head.chunk_size() {
        let (_, _) = head.split(size, ptr);
        //next.write_to(next_addr);
    }
}

unsafe fn write_zone_header_pair(zone: &Zone, header: &Header) {
    let base = zone.base;
    base.write(zone.next);
    base.add(1).write(header.fields);
}

impl Kalloc {
    /// The virtual memory kernel allocator requires at least
    /// one page to use as a `Zone`. On initialization, create
    /// a new zone and initialize the memory with a zone and
    /// chunk header.
    pub fn new(start: Page) -> Self {
        // Make sure start of allocation pool is page aligned.
        assert_eq!(start.addr.addr() & (PAGE_SIZE - 1), 0);
        // New page is the first zone in the Kalloc pool.
        let zone = Zone::new(start.addr);
        let head = Header::new(MAX_CHUNK_SIZE);
        unsafe {
            write_zone_header_pair(&zone, &head);
        }
        Kalloc {
            head: start.addr,
            end: start.addr.map_addr(|addr| addr + 0x1000),
        }
    }

    fn grow_pool(&self, tail: &mut Zone) -> Result<(Zone, Header), VmError> {
        let page = palloc()?;
        unsafe {
            tail.write_next(page.addr);
        }
        let zone = Zone::new(page.addr);
        let head = Header::new(MAX_CHUNK_SIZE);
        unsafe {
            write_zone_header_pair(&zone, &head);
        }
        Ok((zone, head))
    }

    fn shrink_pool(&self, mut drop_zone: Zone) {
        if drop_zone.base != self.head {
            let mut curr_ptr = self.head;
            //let mut curr_zone = Zone::from(curr_ptr);

            loop {
                let curr_zone = Zone::from(curr_ptr);

                if let Some(next_zone) = curr_zone.next_zone() {
                    if drop_zone.base == next_zone.base {
                        drop_zone.free_self(curr_zone);
                        return;
                    } else {
                        curr_ptr = next_zone.base;
                    }
                } else {
                    break;
                }
            }
            panic!(
                "Tried to free zone after: {:?}. Not in the pool...",
                curr_ptr
            );
        }
    }

    /// Finds the first fit for the requested size.
    /// 1. Scan first zone from first to last for a free chunk that fits.
    /// 2a. If success: Return chunk's starting address (*mut usize).
    /// 2b. Else, move to next zone and go back to step 1.
    /// 3. If no zone had a fit, then try to allocate a new zone (palloc()).
    /// 4. If 3. success, allocate from first chunk in new page. Else, fail with OOM.
    pub fn alloc(&mut self, size: usize) -> Result<*mut usize, KallocError> {
        if size == 0 {
            return Err(KallocError::Void);
        }
        // Round to a 8 byte granularity
        let size = if size % 8 != 0 { (size + 7) & !7 } else { size };

        let curr = self.head;
        let end = self.end.map_addr(|addr| addr - 0x1000);
        let mut zone = Zone::from(curr);
        let mut trail = zone;

        while zone.base <= end {
            if let Some(ptr) = zone.scan(size) {
                return Ok(ptr);
            } else {
                zone = match zone.next_zone() {
                    Some(zone) => zone,
                    None => {
                        if let Ok((mut zone, mut head)) = self.grow_pool(&mut trail) {
                            let head_ptr = zone.base.map_addr(|addr| addr + ZONE_SIZE);
                            alloc_chunk(size, head_ptr, &mut zone, &mut head);
                            return Ok(head_ptr.map_addr(|addr| addr + HEADER_SIZE));
                        } else {
                            return Err(KallocError::OOM);
                        }
                    }
                }
            };
        }
        Err(KallocError::OOM)
    }

    /// 1. Calculate the header offset from the data pointer.
    /// 2. Calculate the zone offset from the data pointer.
    /// 3. Check if zone refs count is 0, if so, release zone.
    /// 4. If zone refs count != 0, try to merge this freed chunk.
    pub fn free<T>(&mut self, ptr: *mut T) {
        let ptr: *mut usize = ptr.cast();
        // Assume that round down to nearest page is the current zone base addr.
        let mut zone = Zone::from(ptr.map_addr(|addr| addr & !(PAGE_SIZE - 1)));
        let head_ptr = ptr.map_addr(|addr| addr - HEADER_SIZE);
        let mut head = Header::from(head_ptr);
        assert!(!head.is_free(), "Kalloc double free.");
        head.set_unused();

        let mut chunk_merge_flag = false;
        if let Ok(count) = zone.decrement_refs() {
            if count == 0 {
                // this is costly, as it's a list traversal
                self.shrink_pool(zone);
            } else {
                chunk_merge_flag = true;
            }
        } else {
            panic!("Negative zone refs count: {}", zone.get_refs())
        }

        if chunk_merge_flag {
            let next_ptr = ptr.map_addr(|addr| addr + head.chunk_size());
            let next = Header::from(next_ptr);
            if next.is_free() && next_ptr < zone.base.map_addr(|addr| addr + 0x1000) {
                // back to back free, merge
                //head.set_size(head.chunk_size() + HEADER_SIZE + next.chunk_size())
                head.merge(next, next_ptr);
            }
        }
        head.write_to(head_ptr);
    }
}
