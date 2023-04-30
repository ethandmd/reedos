//! Page table
// VA: 39bits, PA: 56bits
// PTE size = 8 bytes
use crate::hw::param::*;
use crate::hw::riscv::*;
use crate::vm::*;
use core::assert;

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

pub type VirtAddress = *mut usize;
pub type PhysAddress = *mut usize;
type PTEntry = usize;
/// Supervisor Address Translation and Protection.
/// Section 4.1.12 of risc-v priviliged ISA manual.
pub type SATPAddress = usize;

/// Abstraction of a page table at a physical address.
/// Notice we didn't use a rust array here, instead
/// implementing our own indexing methods, functionally
/// similar to that of an array.
#[derive(Copy, Clone)]
#[repr(C)]
pub struct PageTable {
    pub base: PhysAddress, // Page Table located at base address.
}

#[inline(always)]
fn vpn(ptr: VirtAddress, level: usize) -> usize {
    ptr.addr() >> (12 + 9 * level) & 0x1FF
}

#[inline(always)]
fn pte_to_phy(pte: PTEntry) -> PhysAddress {
    ((pte >> 10) << 12) as *mut usize
}

#[inline(always)]
fn phy_to_pte(ptr: PhysAddress) -> PTEntry {
    ((ptr.addr()) >> 12) << 10
}

macro_rules! PteGetFlag {
    ($pte:expr, $flag:expr) => {
        ($pte) & $flag != 0
    };
}

macro_rules! PteSetFlag {
    ($pte:expr, $flag:expr) => {
        (($pte) | $flag)
    };
}

#[inline(always)]
fn phy_to_satp(ptr: PhysAddress) -> usize {
    (1 << 63) | (ptr.addr() >> 12)
}

macro_rules! PageAlignDown {
    ($p:expr) => {
        ($p).map_addr(|addr| addr & !(PAGE_SIZE - 1))
    };
}

// Read the memory at location self + index * 8 bytes
unsafe fn get_phy_offset(phy: PhysAddress, index: usize) -> *mut PTEntry {
    phy.byte_add(index * 8)
}

fn set_pte(pte: *mut PTEntry, contents: PTEntry) {
    unsafe {
        pte.write_volatile(contents);
    }
}

fn read_pte(pte: *mut PTEntry) -> PTEntry {
    unsafe { pte.read_volatile() }
}

impl From<PTEntry> for PageTable {
    fn from(pte: PTEntry) -> Self {
        PageTable {
            base: pte_to_phy(pte),
        }
    }
}

impl PageTable {
    pub fn new(addr: *mut usize) -> Self {
        Self {
            base: addr as PhysAddress,
        }
    }

    fn index_mut(&self, idx: usize) -> *mut PTEntry {
        assert!(idx < PTE_TOP);
        unsafe { get_phy_offset(self.base, idx) }
    }
    pub fn write_satp(&self) {
        flush_tlb();
        write_satp(phy_to_satp(self.base));
        flush_tlb();
    }
}

// Get the address of the PTE for va given the page table pt.
// Returns Either PTE or None, callers responsibility to use PTE
// or allocate a new page.
unsafe fn walk(pt: PageTable, va: VirtAddress, alloc_new: bool) -> Result<*mut PTEntry, VmError> {
    let mut table = pt;
    assert!(va.addr() < VA_TOP);
    for level in (1..3).rev() {
        let idx = vpn(va, level);
        let next: *mut PTEntry = table.index_mut(idx);
        table = match PteGetFlag!(*next, PTE_VALID) {
            true => PageTable::from(*next),
            false => {
                if alloc_new {
                    match PAGEPOOL.get_mut().unwrap().palloc() {
                        Ok(pg) => {
                            *next = PteSetFlag!(phy_to_pte(pg.addr), PTE_VALID);
                            PageTable::from(phy_to_pte(pg.addr))
                        }
                        Err(e) => return Err(e),
                    }
                } else {
                    return Err(VmError::PallocFail);
                }
            }
        };
    }
    // Last, return PTE leaf. Assuming we are all using 4K pages right now.
    // Caller's responsibility to check flags.
    let idx = vpn(va, 0);
    Ok(table.index_mut(idx))
}

/// Helper for making flags for page_map for unpriviledged processes
pub fn user_process_flags(r: bool, w: bool, e: bool) -> usize {
    PTE_USER |
    if r {PTE_READ} else {0} |
    if w {PTE_WRITE} else {0} |
    if e {PTE_EXEC} else {0}
}

/// Helper for making flags for page_map for priviledged processes
pub fn kernel_process_flags(r: bool, w: bool, e: bool) -> usize {
    0 |
    if r {PTE_READ} else {0} |
    if w {PTE_WRITE} else {0} |
    if e {PTE_EXEC} else {0}
}

/// Maps some number of pages into the VM given by pt of byte length
/// size.
pub fn page_map(
    pt: PageTable,
    va: VirtAddress,
    pa: PhysAddress,
    size: usize,
    flag: usize,
) -> Result<(), VmError> {
    // Round down to page aligned boundary (multiple of pg size).
    let mut start = PageAlignDown!(va);
    let mut phys = pa;
    let end = PageAlignDown!(va.map_addr(|addr| addr + (size - 1)));

    while start <= end {
        let walk_addr = unsafe { walk(pt, start, true) };
        match walk_addr {
            Err(e) => {
                return Err(e);
            }
            Ok(pte_addr) => {
                if read_pte(pte_addr) & PTE_VALID != 0 {
                    return Err(VmError::PallocFail);
                }
                set_pte(pte_addr, PteSetFlag!(phy_to_pte(phys), flag | PTE_VALID));
                start = start.map_addr(|addr| addr + PAGE_SIZE);
                phys = phys.map_addr(|addr| addr + PAGE_SIZE);
            }
        }
    }

    Ok(())
}

/// Create the kernel page table with 1:1 mappings to physical memory.
/// First allocate a new page for the kernel page table.
/// Next, map memory mapped I/O devices to the kernel page table.
/// Then map the kernel .text, .data, .rodata and .bss sections.
/// Additionally, map a stack+guard page for each hart.
/// Finally map, the remaining physical memory to kernel virtual memory as
/// the kernel 'heap'.
pub fn kpage_init() -> Result<PageTable, VmError> {
    let base = unsafe {
        PAGEPOOL
            .get_mut()
            .unwrap()
            .palloc()
            .expect("Couldn't allocate root kernel page table.")
    };
    //log!(Debug, "Kernel page table base addr: {:#02x}", base.addr.addr());
    let kpage_table = PageTable {
        base: base.addr as *mut usize,
    };

    page_map(
        kpage_table,
        UART_BASE as *mut usize,
        UART_BASE as *mut usize,
        PAGE_SIZE,
        PTE_READ | PTE_WRITE,
    )?;
    log!(Debug, "Successfully mapped UART into kernel pgtable...");

    page_map(
        kpage_table,
        PLIC_BASE as *mut usize,
        PLIC_BASE as *mut usize,
        PLIC_SIZE,
        PTE_READ | PTE_WRITE,
    )?;
    log!(Debug, "Successfully mapped PLIC into kernel pgtable...");
    
    page_map(
        kpage_table,
        VIRTIO_BASE as *mut usize,
        VIRTIO_BASE as *mut usize,
        VIRTIO_SIZE,
        PTE_READ | PTE_WRITE,
    )?;
    log!(Debug, "Successfully mapped PLIC into kernel pgtable...");
    page_map(
        kpage_table,
        DRAM_BASE,
        DRAM_BASE as *mut usize,
        text_end().addr() - DRAM_BASE.addr(),
        PTE_READ | PTE_EXEC,
    )?;
    log!(
        Debug,
        "Succesfully mapped kernel text into kernel pgtable..."
    );

    // assert!(trampoline_end() as usize - trampoline_start() as usize == 0x1000,
    //         "Trampoline page is not a page!");
    // // map once on top of pa
    // page_map(
    //     kpage_table,
    //     trampoline_start(),
    //     trampoline_start(),
    //     0x1000,                 // checked above
    //     PTE_READ | PTE_EXEC,
    // )?;
    // // map again as top page
    // page_map(
    //     kpage_table,
    //     trampoline_target(),
    //     trampoline_start(),
    //     0x1000,                 // checked above
    //     PTE_READ | PTE_EXEC,
    // )?;
    // log!(
    //     Debug,
    //     "Succesfully mapped trampoline page into kernel pgtable..."
    // );

    page_map(
        kpage_table,
        text_end(),
        text_end() as *mut usize,
        rodata_end().addr() - text_end().addr(),
        PTE_READ,
    )?;
    log!(
        Debug,
        "Succesfully mapped kernel rodata into kernel pgtable..."
    );

    page_map(
        kpage_table,
        rodata_end(),
        rodata_end() as *mut usize,
        data_end().addr() - rodata_end().addr(),
        PTE_READ | PTE_WRITE,
    )?;
    log!(
        Debug,
        "Succesfully mapped kernel data into kernel pgtable..."
    );

    // This maps hart 0, 1 stack pages in opposite order as entry.S. Shouln't necessarily be a
    // problem.
    let base = stacks_start();
    for s in 0..NHART {
        let stack = unsafe { base.byte_add(PAGE_SIZE * (1 + s * 3)) };
        page_map(
            kpage_table,
            stack,
            stack,
            PAGE_SIZE * 2,
            PTE_READ | PTE_WRITE,
        )?;
        log!(
            Debug,
            "Succesfully mapped kernel stack {} into kernel pgtable...",
            s
        );
    }

    // This maps hart 0, 1 stack pages in opposite order as entry.S. Shouln't necessarily be a
    // problem.
    let base = intstacks_start();
    for i in 0..NHART {
        let m_intstack = unsafe { base.byte_add(PAGE_SIZE * (1 + i * 4)) };
        // Map hart i m-mode handler.
        page_map(
            kpage_table,
            m_intstack,
            m_intstack,
            PAGE_SIZE,
            PTE_READ | PTE_WRITE,
        )?;
        // Map hart i s-mode handler
        let s_intstack = unsafe { m_intstack.byte_add(PAGE_SIZE * 2) };
        page_map(
            kpage_table,
            s_intstack,
            s_intstack,
            PAGE_SIZE,
            PTE_READ | PTE_WRITE,
        )?;
        log!(
            Debug,
            "Succesfully mapped interrupt stack for hart {} into kernel pgtable...",
            i
        );
    }

    page_map(
        kpage_table,
        bss_start(),
        bss_start(),
        bss_end().addr() - bss_start().addr(),
        PTE_READ | PTE_WRITE,
    )?;
    log!(Debug, "Succesfully mapped kernel bss...");

    page_map(
        kpage_table,
        bss_end(),
        bss_end(),
        memory_end().addr() - bss_end().addr(),
        PTE_READ | PTE_WRITE,
    )?;
    log!(Debug, "Succesfully mapped kernel heap...");

    Ok(kpage_table)
}
