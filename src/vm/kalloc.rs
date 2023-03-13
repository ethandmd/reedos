use core::mem::size_of;

use crate::hw::param::PAGE_SIZE;
use super::{palloc::Page, VmError};

const MAX_CHUNK_SIZE: usize = 4088; // PAGE_SIZE - HEADER_SIZE = 4096 - 8 = 4088.
const HEADER_SIZE: usize = size_of::<Header>();
const HEADER_USED: usize = 0x1000;

// 8-byte minimum allocation size,
// 4096-byte maximum allocation size.
// Guarantee that address of header + size = start of data.
// Size must be <= 4088 Bytes.
// Bits 0-11 are size (2^0 - (2^12 - 1))
// Bit 12 is free/used.
#[repr(C)]
struct Header {
    fields: usize, // Could be a union?
}

impl From<*mut usize> for Header {
    fn from(src: *mut usize) -> Self {
        let fields = unsafe { src.read() };
        Header { fields }
    }
}

impl Header {
    fn chunk_size(&self) -> usize {
        self.fields & 0xFFF
    }

    fn is_free(&self) -> bool {
        self.fields & !HEADER_USED == 0
    }

    fn set_used(&mut self) {
        self.fields = self.fields | HEADER_USED;
    }

    // Clear size bits. Set size bits to size.
    fn set_size(&mut self, size: usize) {
        self.fields = (self.fields & !MAX_CHUNK_SIZE) | size;
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
        let next_header = Header { fields: next_size };
        next_header.write_to(next_addr);
        (next_header, next_addr)
    }
}

struct Kalloc {
    head: *mut usize, // Address of next free header.
    end: *mut usize,
}

impl Kalloc {
    fn new(start: Page) -> Self {
        // Make sure start of allocation pool is page aligned.
        assert_eq!(start.addr.addr() & (PAGE_SIZE - 1), 0);
        let head = Header { fields: MAX_CHUNK_SIZE };
        head.write_to(start.addr);

        Kalloc {
            head: start.addr,
            end: unsafe { start.addr.byte_add(0x1000) },
        }
    }

    fn alloc(&mut self, size: usize) -> Result<*mut usize, VmError> {
        // Start tracks address of each header.
        let mut start = self.head;
        let mut head = Header::from(start);

        // Remove redundancy + use some helper fns.
        while start != self.end {
            let chunk_size = head.chunk_size();
            if chunk_size < size {
                start = start.map_addr(|addr| addr + HEADER_SIZE + chunk_size);
                head = Header::from(start);
            } else if !head.is_free() {
                start = start.map_addr(|addr| addr + HEADER_SIZE + chunk_size);
                head = Header::from(start);
            } else {
                head.set_used();
                let (next, next_addr) = head.split(size, start);
                next.write_to(next_addr);
                return Ok(start.map_addr(|addr| addr + size))
            }
        }
        Err(VmError::Koom)
    }
}
