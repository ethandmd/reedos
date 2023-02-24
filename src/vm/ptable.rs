// VA: 39bits, PA: 56bits
// PTE size = 8 bytes
use core::assert;
use core::ops::Index;
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

type VirtAddress = usize;
type PhysAddress = *mut usize;
type PTEntry = usize;
pub type SATPAddress = usize;

#[derive(Copy, Clone)]
#[repr(C)]
struct PageTable {
    base: PhysAddress, // Page Table located at base address.
}

enum VmError{} // Custom error type, may remove later.

macro_rules! vpn {
    ($p:expr, $l:expr) => {
        $p >> (12 + 9 * $l) & 0x1FF
    }
}

macro_rules! PteToPhy {
    ($p:expr) => {
        (($p >> 10) << 12) as *mut usize
    }
}

macro_rules! PhyToPte {
    ($p:expr) => {
        (($p >> 12) << 10)
    }
}

macro_rules! PteGetFlag {
    ($pte:expr, $flag:expr) => {
        $pte & $flag != 0
    }
}

macro_rules! PteSetFlag {
    ($pte:expr, $flag:expr) => {
            ($pte | $flag)
    }
}

macro_rules! PhyToSATP {
    ($pte:expr) => {
        (1 << 63) | ($pte >> 12)
    }
}

// Read the memory at location self + index * 8 bytes
unsafe fn read_phy_offset(phy: PhysAddress, index: usize) ->  usize {
    phy.byte_add(index * 8).read_volatile()
}

fn set_pte(phy: &PhysAddress, pte: PTEntry) {
    unsafe {
        phy.write_volatile(pte);
    }
}           

impl From<PTEntry> for PageTable {
    fn from(pte: PTEntry) -> Self {
        PageTable { base: PteToPhy!(pte) }
    }
}

impl Index<usize> for PageTable {
    type Output = PTEntry;

    fn index(&self, idx: usize) -> &Self::Output {
        assert!(idx < PTE_TOP);
        unsafe {
            &PhyToPte!(read_phy_offset(self.base, idx))
        }
    }
}


// Get the address of the PTE for va given the page table pt.
// Returns Either PTE or None, callers responsibility to use PTE 
// or allocate a new page.
fn walk(pt: &PageTable, va: VirtAddress) -> Option<PTEntry> {
    let mut table = pt.clone();
    assert!(va < VA_TOP);
    for level in (1..3).rev() {
        let idx = vpn!(va, level);
        let next: PTEntry = table[idx];
        table = match PteGetFlag!(next, PTE_VALID) {
            true => { PageTable::from(next) },
            false => {return None }
        };
    }
    // Last, return PTE leaf. Assuming we are all using 4K pages right now.
    // Caller's responsibility to check flags.
    let idx = vpn!(va, 0);
    Some(table[idx])
}


/// Returns the next page table one level down, possibly allocating if
/// it does not exist
///
/// TODO make sure these are the right flags to write for new page table level
fn get_next_level_or_alloc(pool: &mut Kpools, table: &PageTable, idx:usize) -> PageTable {
    let mut l2_entry = table[idx];
    if !PteGetFlag!(l2_entry, PTE_VALID) {
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
    assert!(va < VA_TOP);
    let l3_range = (vpn!(va, 3), (va + vpn!(4096*num_pages, 3)));
    // not that these are not really contiguous, you need to nagivate
    // the tree, these are just the outermost bounds at each level
    // ASSUMING you are already at the bound of the higher level
    for i3 in l3_range.0..l3_range.1 { 
        let mut l2_range = (vpn!(va, 2), (va + (vpn!(4096*num_pages, 2))));
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
            let mut l1_range = (vpn!(va,1), (va + (vpn!(4096*num_pages, 1))));
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
    let start = va & !(4096 - 1);
    let num_pages = size >> 12;
    if start != va {
        log!(Warning, "page_map rounded virtual address down to a page size multiple.");
    }
    if num_pages != size << 12 {
        log!(Warning, "page_map rounded size down to a page size multiple.");
    }

    ensure_valid_walk(pool, pt, va, num_pages);
    
    for i in 0..num_pages {
        match walk(pt, start + (4096 * i)) {
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
    //     match walk(pt, start)) {
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
    let base = pool.palloc(1).unwrap() as *mut usize;
    let mut kpage_table = PageTable { base };

    unsafe {
        _ = page_map(
            pool,
            &mut kpage_table, 
            UART_BASE, 
            UART_BASE as *mut usize, 
            PAGE_SIZE, 
            PTE_READ | PTE_WRITE);

        _ = page_map(
            pool,
            &mut kpage_table, 
            DRAM_BASE, 
            DRAM_BASE as *mut usize, 
            __text_end - DRAM_BASE, 
            PTE_READ | PTE_EXEC);

        _ = page_map(
            pool,
            &mut kpage_table, 
            __text_end, 
            __text_end as *mut usize, 
            __memory_end - __text_end, 
            PTE_READ | PTE_WRITE);
    }

    write_satp(PhyToSATP!(unsafe { *kpage_table.base }));
}




