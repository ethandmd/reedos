// VA: 39bits, PA: 56bits
// PTE size = 8 bytes
use core::assert;

const VA_TOP: usize = 1 << (27 + 12); // 2^27 VPN + 12 Offset
const PTE_TOP: usize = 512; // 4Kb / 8 byte PTEs = 512 PTEs / page!

#[derive(PartialOrd, PartialEq)]
#[repr(C)]
struct VirtAddress(usize);

#[repr(C)]
struct PhysAddress(usize);

#[derive(Copy, Clone)]
#[repr(C)]
struct PTEntry(usize);

#[repr(C)]
struct PageTable {
    base: PhysAddress,
}

struct VmError;

impl VirtAddress {
    fn vpn(&self, level: usize) -> usize {
        self.0 >> (12 + 9*level) & 0x1FF
    }

    fn off(&self) -> usize {
        self.0 & 0xFFF
    }
}

impl From<PTEntry> for PhysAddress {
    fn from(pte: PTEntry) -> Self {
        // 10-bit reserved / flag, 12-bit offset
        PhysAddress((pte.0 >> 10) << 12)
    }
}

impl PhysAddress {
    unsafe fn offset(&self, index: usize) -> usize {
        let addr = self.0 as *mut usize;
        addr.byte_add(index * 8).read_volatile()
    }
}

impl From<usize> for PTEntry {
    fn from(ptr: usize) -> Self {
        PTEntry(ptr)
    }
}

impl From<PTEntry> for PageTable {
    fn from(pte: PTEntry) -> Self {
        let addr = PhysAddress::from(pte);
        PageTable { base: addr }
    }
}

impl PageTable {
    fn index(&self, index: usize) -> PTEntry {
        assert!(index < PTE_TOP);
        unsafe {
            PTEntry::from(self.base.offset(index))
        }
    }
}

impl PTEntry {
    fn flag(&self, shift: u8) -> bool { self.0 & (1 << shift) != 0 }
    fn valid(&self) -> bool { self.flag(0) }
    fn read(&self) -> bool { self.flag(1) }
    fn write(&self) -> bool { self.flag(2) }
    fn exec(&self) -> bool { self.flag(3) }
    fn user(&self) -> bool { self.flag(4) }
    fn global(&self) -> bool { self.flag(5) }
    fn accesssed(&self) -> bool { self.flag(6) }
    fn dirty(&self) -> bool { self.flag(7) }
}

// Get the address of the PTE for va given the page table pt.
// Returns Either PTE or None, callers responsibility to use PTE 
// or allocate a new page.
fn walk(mut pt: PageTable, va: VirtAddress) -> Option<PTEntry> {
    assert!(va.0 < VA_TOP);
    for level in (1..3).rev() {
        let idx = va.vpn(level);
        let next: PTEntry = pt.index(idx);
        match next.valid() {
            true => {pt = PageTable::from(next); },
            false => {return None }
        };
    }
    let idx = va.vpn(0);
    Some(pt.index(idx))
}
