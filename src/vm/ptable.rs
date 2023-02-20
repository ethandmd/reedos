// VA: 39bits, PA: 56bits
// PTE size = 8 bytes
use core::assert;
use crate::vm::palloc::Kpools;

const VA_TOP: usize = 1 << (27 + 12); // 2^27 VPN + 12 Offset
const PTE_TOP: usize = 512; // 4Kb / 8 byte PTEs = 512 PTEs / page!
const PTE_VALID: usize = 1 << 0;
const PTE_READ: usize = 1 << 1;
const PTE_WRITE: usize = 1 << 2;
const PTE_EXEC: usize = 1 << 3;
const PTE_USER: usize = 1 << 4;
const PTE_GLOBAL: usize = 1 << 5;
const PTE_ACCESSED: usize = 1 << 6;
const PTE_DIRTY: usize = 1 << 7;

#[repr(C)]
struct VirtAddress(usize); // Top 3 * 9 bits for VPN[i], bottom 12 are page offset.

#[derive(Copy, Clone)]
#[repr(C)]
struct PhysAddress(usize); // Top 44 bits are PPN[i], bottom 12 are PO.

#[derive(Copy, Clone)]
#[repr(C)]
struct PTEntry(usize); // Top 10 bits reserved, next 44 PPN[i], bottom 10 flags.

#[derive(Copy, Clone)]
#[repr(C)]
struct PageTable {
    base: PhysAddress, // Page Table located at base address.
}

struct VmError; // Custom error type, may remove later.

impl From<usize> for VirtAddress {
    fn from(ptr: usize) -> Self {
        VirtAddress(ptr)
    }
}

impl core::ops::Add for VirtAddress {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        Self(self.0 + rhs.0)
    }
}

impl VirtAddress {
    // Get the VPN by level for offset into page table
    fn vpn(&self, level: usize) -> usize {
        self.0 >> (12 + 9*level) & 0x1FF
    }
}

impl From<PTEntry> for PhysAddress {
    fn from(pte: PTEntry) -> Self {
        // 10-bit reserved / flag, 12-bit offset
        PhysAddress((pte.0 >> 10) << 12)
    }
}

impl core::ops::Add<usize> for PhysAddress {
    type Output = Self;
    fn add(self, rhs: usize) -> Self {
        Self(self.0 + rhs)
    }
}

impl PhysAddress {
    // Read the memory at location self + index * 8 bytes
    unsafe fn read_offset(&self, index: usize) -> usize {
        let addr = self.0 as *mut usize;
        addr.byte_add(index * 8).read_volatile()
    }

    fn to_pte(&self, flag: usize) -> PTEntry {
        PTEntry( ((self.0 >> 12) << 10) | flag )
    }
}

impl From<usize> for PTEntry {
    fn from(ptr: usize) -> Self {
        PTEntry(ptr)
    }
}

impl PTEntry {
    fn set(&self, pte: PTEntry) {
        let base = self.0 as *mut usize;
        unsafe {
            base.write_volatile(pte.0);
        }
    }
}
            

impl From<PTEntry> for PageTable {
    fn from(pte: PTEntry) -> Self {
        let addr = PhysAddress::from(pte);
        PageTable { base: addr }
    }
}

impl PageTable {
    // Get PTE at index bytes from base PhysAddr of this page table.
    fn index(&self, index: usize) -> PTEntry {
        assert!(index < PTE_TOP);
        unsafe {
            PTEntry::from(self.base.read_offset(index))
        }
    }
}

impl PTEntry {
    // We'll see how this ages.
    fn flag(&self, flag: usize) -> bool {
        unsafe {
            (self.0 as *mut usize).read_volatile() & flag != 0
        }
    }
    fn valid(&self) -> bool { self.flag(PTE_VALID) }
    fn read(&self) -> bool { self.flag(PTE_READ)  }
    fn write(&self) -> bool { self.flag(PTE_WRITE) }
    fn exec(&self) -> bool { self.flag(PTE_EXEC) }
    fn user(&self) -> bool { self.flag(PTE_USER) }
    fn global(&self) -> bool { self.flag(PTE_GLOBAL) }
    fn accesssed(&self) -> bool { self.flag(PTE_ACCESSED) }
    fn dirty(&self) -> bool { self.flag(PTE_DIRTY) }
}

// Get the address of the PTE for va given the page table pt.
// Returns Either PTE or None, callers responsibility to use PTE 
// or allocate a new page.
fn walk(pt: &PageTable, va: VirtAddress) -> Option<PTEntry> {
    let mut table = pt.clone();
    assert!(va.0 < VA_TOP);
    for level in (1..3).rev() {
        let idx = va.vpn(level);
        let next: PTEntry = table.index(idx);
        table = match next.valid() {
            true => { PageTable::from(next) },
            false => {return None }
        };
    }
    // Last, return PTE leaf. Assuming we are all using 4K pages right now.
    // Caller's responsibility to check flags.
    let idx = va.vpn(0);
    Some(table.index(idx))
}

fn map(kpool: &mut Kpools, pt: &mut PageTable, va: VirtAddress, mut pa: PhysAddress, size: usize, flag: usize) -> Result<(), VmError> {
    // Round down to next page aligned boundary (multiple of pg size).
    let mut start = va.0 & !(4096 - 1);
    let end = (va.0 + size) & !(4096 - 1);

    while start < end {
        match walk(pt, VirtAddress::from(start)) {
            Some(pte) => { pte.set(pa.to_pte(flag | PTE_VALID)); }
            None => {
                // Allocate a new page
                match kpool.palloc() {
                    Some(ptr) => { PTEntry::from(ptr as usize).set(pa.to_pte(flag | PTE_VALID)); },
                    None => { return Err(VmError); },
                };
            }
        };
        start += 4096;
        pa = pa + 4096;
    }
    Ok(())
}





