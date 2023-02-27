// VA: 39bits, PA: 56bits
// PTE size = 8 bytes
use core::assert;
use crate::vm::*;
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

type VirtAddress = *mut usize;
type PhysAddress = *mut usize;
type PTEntry = usize;
pub type SATPAddress = usize;

#[derive(Copy, Clone)]
#[repr(C)]
struct PageTable {
    base: PhysAddress, // Page Table located at base address.
}

macro_rules! vpn {
    ($p:expr, $l:expr) => {
        (($p).addr()) >> (12 + 9 * $l) & 0x1FF
    }
}

macro_rules! PteToPhy {
    ($p:expr) => {
        ((($p) >> 10) << 12) as *mut usize
    }
}

macro_rules! PhyToPte {
    ($p:expr) => {
        (((($p).addr()) >> 12) << 10)
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
        (1 << 63) | ((($pte).addr()) >> 12)
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
unsafe fn walk(pt: &PageTable, va: VirtAddress, alloc_new: bool) -> Result<*mut PTEntry, VmError> {
    let mut table = pt.clone();
    assert!(va.addr() < VA_TOP);
    for level in (1..3).rev() {
        let idx = vpn!(va, level);
        let next: *mut PTEntry = table.index_mut(idx);
        table = match PteGetFlag!(*next, PTE_VALID) {
            true => { PageTable::from(*next) },
            false => {
                if alloc_new {
                    match (*PAGEPOOL).palloc() {
                        Ok(pg) => {
                            *next = PteSetFlag!(PhyToPte!(pg.addr), PTE_VALID);
                            PageTable::from(PhyToPte!(pg.addr))
                        }
                        Err(e) => { return Err(e) }
                    }
                } else { 
                    return Err(VmError::PallocFail); 
                }
            }
        };
    }
    // Last, return PTE leaf. Assuming we are all using 4K pages right now.
    // Caller's responsibility to check flags.
    let idx = vpn!(va, 0);
    Ok(table.index_mut(idx))
}

/// Maps some number of pages into the VM given by pt of byte length
/// size. 
///
/// Rounds down va and size to page size multiples. 
fn page_map(pt: &mut PageTable, va: VirtAddress, pa: PhysAddress, size: usize, flag: usize) -> Result<(), VmError> {
    // Round up to next page aligned boundary (multiple of pg size).
    let start = va.map_addr(|addr| addr + (PAGE_SIZE - 1) & !(PAGE_SIZE - 1));
    let num_pages = ((size + (PAGE_SIZE - 1)) & !(PAGE_SIZE - 1)) >> 12;
   
    for i in 0..num_pages {
        match unsafe { walk(pt, start.map_addr(|addr| addr + (4096 * i)), true) } {
            Ok(pte) => {
                set_pte(pte, PteSetFlag!(PhyToPte!(pa.map_addr(|addr| addr + PAGE_SIZE*i)), flag | PTE_VALID)); 
            },
            Err(e) => {
                log!(Error, "{:?}", e);
                panic!();
            }
        }
        
    }
    Ok(())
}

// Initialize kernel page table 
pub fn kpage_init() {
    let base = unsafe { (*PAGEPOOL).palloc().expect("Couldn't allocate root kernel page table.") };
    //log!(Debug, "Kernel page table base addr: {:#02x}", base.addr.addr());
    let mut kpage_table = PageTable { base: base.addr as *mut usize };

    _ = page_map(
        &mut kpage_table, 
        UART_BASE as *mut usize, 
        UART_BASE as *mut usize, 
        PAGE_SIZE, 
        PTE_READ | PTE_WRITE);
    log!(Debug, "Successfully mapped UART into kernel pgtable...");

    _ = page_map(
        &mut kpage_table, 
        DRAM_BASE, 
        DRAM_BASE as *mut usize, 
        text_end().addr() - DRAM_BASE.addr(), 
        PTE_READ | PTE_EXEC);
    log!(Debug, "Succesfully mapped kernel text into kernel pgtable...");

    _ = page_map(
        &mut kpage_table, 
        text_end(), 
        text_end(), 
        dram_end().addr() - text_end().addr(), 
        PTE_READ | PTE_WRITE);
    log!(Debug, "Succesfully mapped kernel heap...");

    // SFENCE.VMA Instruction here? After?
    write_satp(PhyToSATP!(kpage_table.base));
}
