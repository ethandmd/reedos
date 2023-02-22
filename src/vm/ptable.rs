// VA: 39bits, PA: 56bits
// PTE size = 8 bytes
use core::assert;
use crate::vm::palloc::Kpools;
use crate::hw::param::*;
use crate::hw::riscv::write_satp;

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

pub struct SATPAddress(usize);

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

impl Clone for VirtAddress {
    fn clone(&self) -> Self {
        VirtAddress(self.0)
    }
}

impl From<PTEntry> for PhysAddress {
    fn from(pte: PTEntry) -> Self {
        // 10-bit reserved / flag, 12-bit offset
        PhysAddress((pte.0 >> 10) << 12)
    }
}

impl From<usize> for PhysAddress {
    fn from(ptr: usize) -> Self {
        PhysAddress(ptr)
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

    fn write_flags(&mut self, flags: usize) {
        unsafe {
            (self.0 as *mut usize).write_volatile(flags & (PTE_VALID |
                                                            PTE_READ |
                                                            PTE_WRITE |
                                                            PTE_EXEC |
                                                            PTE_USER |
                                                            PTE_GLOBAL |
                                                            PTE_ACCESSED |
                                                            PTE_DIRTY));
        }
    }
}

impl From<usize> for SATPAddress {
    fn from(ptr: usize) -> Self {
        SATPAddress( (1 << 63) | ( ptr >> 12)) // (MODE = 8, for Sv39) | PPN without offset)
    }
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


/// Returns the next page table one level down, possibly allocating if
/// it does not exist
///
/// TODO make sure these are the right flags to write for new page table level
fn get_next_level_or_alloc(pool: &mut Kpools, table: &PageTable, idx:usize) -> PageTable {
    let mut l2_entry = table.index(idx);
    if !l2_entry.valid() {
        // need to extend the tree
        match pool.palloc(1) {
            Some(page) => {
                l2_entry.set(PTEntry(page as usize));
                l2_entry.write_flags(PTE_VALID | PTE_READ | PTE_WRITE);
            },
            None => {
                log!(Error, "Couldn't allocate a page on new page table expansion.");
                panic!();
            }
        }
    }
    PageTable::from(l2_entry)
}

/// Walks for the given page extent and ensures that all the tables on all the levels are valid, or allocates them if not
fn ensure_valid_walk(pool: &mut Kpools, pt: &PageTable, va: VirtAddress, num_pages: usize) {
    let l3_table = pt.clone();
    assert!(va.0 < VA_TOP);
    let l3_range = (va.vpn(3), (va.clone() + (VirtAddress::from(4096*num_pages))).vpn(3));
    // not that these are not really contiguous, you need to nagivate
    // the tree, these are just the outermost bounds at each level
    // ASSUMING you are already at the bound of the higher level
    for i3 in l3_range.0..l3_range.1 { 
        let mut l2_range = (va.vpn(2), (va.clone() + (VirtAddress::from(4096*num_pages))).vpn(2));
        if i3 != l3_range.0 {
            l2_range.0 = 0;
        }
        if i3 != l3_range.1 {
            l2_range.1 = PTE_TOP;
        }
        // basically if you are in a middle segment, expand the
        // proper side(s) to get the full length for this level
        
        
        let l2_table = get_next_level_or_alloc(pool, &l3_table, i3);
        
        for i2 in l2_range.0..l2_range.1 {
            let mut l1_range = (va.vpn(1), (va.clone() + (VirtAddress::from(4096*num_pages))).vpn(1));
            if i2 != l2_range.0 {
                l1_range.0 = 0;
            }
            if i2 != l2_range.1 {
                l1_range.1 = PTE_TOP;
            }
            // same thing
            
            get_next_level_or_alloc(pool, &l2_table, i2);
            // we don't need to iterate here because after this are
            // leaf nodes, and we only are concerned with making sure
            // the walk itself is valid
        }
    }
}

/// Maps some number of pages into the VM given by pt of byte length
/// size. 
///
/// Rounds down va and size to page size multiples. 
// TODO: Implement VmError
fn page_map(pool: &mut Kpools, pt: &mut PageTable, va: VirtAddress, pa: PhysAddress, size: usize, flag: usize) -> Result<(), VmError> {
    // Round down to next page aligned boundary (multiple of pg size).
    let start = va.0 & !(4096 - 1);
    let num_pages = size >> 12;
    if start != va.0 {
        log!(Warning, "page_map rounded virtual address down to a page size multiple.");
    }
    if num_pages != size << 12 {
        log!(Warning, "page_map rounded size down to a page size multiple.");
    }

    ensure_valid_walk(pool, pt, va, num_pages);
    
    for i in 0..num_pages {
        match walk(pt, VirtAddress::from(start + (4096 * i))) {
            Some(pte) => {
                pte.set((pa + 4096*i).to_pte(flag |PTE_VALID));
            },
            None => {
                log!(Error, "page_map found invalid page on the walk down. Violates assumptions");
                panic!();
            }
        }
        
    }
    Ok(())
    // Old version without assumption. Do not use.
    // while start < end {
    //     // 1. Walk page table and find pte.
    //     match walk(pt, VirtAddress::from(start)) {
    //         // 2. Write PTE and set it as valid
    //         Some(pte) => { pte.set(pa.to_pte(flag | PTE_VALID)); }
    //         // 3. Allocate a new page and do step 2. or fail.
    //         None => {
    //             match pool.palloc() {
    //                 Some(ptr) => { PTEntry::from(ptr as usize).set(pa.to_pte(flag | PTE_VALID)); },
    //                 None => { return Err(VmError); },
    //             };
    //         }
    //     };
    //     // 4. Increase addresses by 1 page per map() iteration.
    //     start += 4096;
    //     pa = pa + 4096;
    // }
    // Ok(())
}

// Initialize kernel page table 
pub fn kpage_init(pool: &mut Kpools) {
    let base = pool.palloc(1).unwrap() as usize;
    let mut kpage_table = PageTable { base: PhysAddress::from(base) };

    unsafe {
        _ = page_map(
            pool,
            &mut kpage_table, 
            VirtAddress::from(UART_BASE), 
            PhysAddress::from(UART_BASE), 
            PAGE_SIZE, 
            PTE_READ | PTE_WRITE);

        _ = page_map(
            pool,
            &mut kpage_table, 
            VirtAddress::from(DRAM_BASE), 
            PhysAddress::from(DRAM_BASE), 
            TEXT_END - DRAM_BASE, 
            PTE_READ | PTE_EXEC);

        _ = page_map(
            pool,
            &mut kpage_table, 
            VirtAddress::from(TEXT_END), 
            PhysAddress::from(TEXT_END), 
            DRAM_END - TEXT_END, 
            PTE_READ | PTE_WRITE);
    }

    write_satp(SATPAddress::from(kpage_table.base.0).0 as u64);
}




