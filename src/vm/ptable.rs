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
        ($p) >> (12 + 9 * $l) & 0x1FF
    }
}

macro_rules! PteToPhy {
    ($p:expr) => {
        ((($p) >> 10) << 12) as *mut usize
    }
}

macro_rules! PhyToPte {
    ($p:expr) => {
        ((($p) >> 12) << 10)
    }
}

macro_rules! PteGetFlag {
    ($pte:expr, $flag:expr) => {
        ($pte) & $flag != 0
    }
}

macro_rules! PteSetFlag {
    ($pte:expr, $flag:expr) => {
            (($pte) | $flag)
    }
}

macro_rules! PhyToSATP {
    ($pte:expr) => {
        (1 << 63) | (($pte) >> 12)
    }
}

// Read the memory at location self + index * 8 bytes
unsafe fn get_phy_offset(phy: PhysAddress, index: usize) ->  *mut PTEntry {
    phy.byte_add(index * 8)
}

fn set_pte(pte: *mut PTEntry, contents: PTEntry) {
    unsafe {
        pte.write_volatile(contents);
    }
}           

impl From<PTEntry> for PageTable {
    fn from(pte: PTEntry) -> Self {
        PageTable { base: PteToPhy!(pte) }
    }
}

impl PageTable {
    fn index_mut(&self, idx: usize) -> *mut PTEntry {
        assert!(idx < PTE_TOP);
        unsafe {
            get_phy_offset(self.base, idx)
        }
    }
}

// Get the address of the PTE for va given the page table pt.
// Returns Either PTE or None, callers responsibility to use PTE 
// or allocate a new page.
unsafe fn walk(pool: &mut Kpools, pt: &PageTable, va: VirtAddress, alloc_new: bool) -> Option<*mut PTEntry> {
    let mut table = pt.clone();
    assert!(va < VA_TOP);
    for level in (1..3).rev() {
        let idx = vpn!(va, level);
        let next: *mut PTEntry = table.index_mut(idx);
        
        table = match PteGetFlag!(*next, PTE_VALID) {
            true => { PageTable::from(*next) },
            false => {
                if alloc_new {
                    match pool.palloc(1) {
                        Some(pg) => { 
                            *next = PhyToPte!(pg as usize);
                            PageTable::from(PhyToPte!(pg as usize))
                        }
                        None => { return None }
                    }
                } else { 
                    return None 
                }
            }
        };
    }
    // Last, return PTE leaf. Assuming we are all using 4K pages right now.
    // Caller's responsibility to check flags.
    let idx = vpn!(va, 0);
    Some(table.index_mut(idx) as *mut PTEntry)
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
    if num_pages << 12 != size {
        log!(Warning, "page_map rounded size down to a page size multiple.");
    }
    
    for i in 0..num_pages {
        match unsafe { walk(pool, pt, start + (4096 * i), true) } {
            Some(pte) => {
                set_pte(pte, PteSetFlag!(PhyToPte!(pa as usize + PAGE_SIZE*i), flag | PTE_VALID)); 
            },
            None => {
                log!(Error, "page_map found invalid page on the walk down. Violates assumptions");
                panic!();
            }
        }
        
    }
    Ok(())
}

// Initialize kernel page table 
pub fn kpage_init(pool: &mut Kpools) {
    let base = pool.palloc(1).expect("Couldn't allocate new page") as *mut usize;
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

    write_satp(PhyToSATP!(kpage_table.base as usize));
}
