//! Process handle and utilities.
// use alloc::boxed::Box;

// extern crate alloc;

// use alloc::boxed::Box;
use alloc::boxed::Box;
use alloc::collections::vec_deque::*;
use core::assert;
use core::mem::size_of;

// use crate::hw::HartContext;
// use crate::trap::TrapFrame;
use crate::vm::ptable::*;
use crate::vm::{request_phys_page, PhysPageExtent};
use crate::file::elf64::*;

pub struct Process {
    id: usize,
    // address_space: BTreeSet<Box<dyn Resource>>, // todo: Balanced BST of Resources
    state: ProcessState,
    pgtbl: PageTable,
    phys_pages: VecDeque<PhysPageExtent>,
    // trapframe: TrapFrame,
    // ctx_regs: HartContext,
}

pub enum ProcessState {
    Unstarted,
    Ready,
    Running,
    Wait,
    Sleep,
    Dead,
}

// this was a method of ELFProgram before, but I think it makes more
// sense as a method of process for scoping reasons
impl Process {
    pub fn populate_pagetable64(&mut self, elf: &ELFProgram) -> Result<(), ELFError>{
        assert!(elf.header.program_entry_size as usize == size_of::<ProgramHeaderSegment64>(),
        "Varying ELF entry size expectations.");

        let num = elf.header.num_program_entries;
        let ptr = elf.header.program_header_pos as *const ProgramHeaderSegment64;
        for i in 0..num {
            let segment = unsafe { *ptr.add(i as usize) };
            if segment.seg_type != ProgramSegmentType::Load { continue; }
            else if segment.vmem_addr < 0x1000 {return Err(ELFError::MappedZeroPage)}

            let n_pages = (segment.size_in_memory + (0x1000 - 1)) / 0x1000;
            let pages = match request_phys_page(n_pages as usize) {
                Ok(p) => {p},
                Err(_) => {return Err(ELFError::FailedAlloc)}
            };
            let flags = user_process_flags(
                segment.flags & PROG_SEG_READ != 0,
                segment.flags & PROG_SEG_WRITE != 0,
                segment.flags & PROG_SEG_EXEC != 0
            );

            match page_map(
                self.pgtbl,
                VirtAddress::from(segment.vmem_addr as *mut usize),
                PhysAddress::from(pages.start() as *mut usize),
                n_pages as usize,
                flags
            ) {
                Ok(_) => {},
                Err(_) => {return Err(ELFError::FailedMap)}
            }
            self.phys_pages.push_back(pages);
        }

        Ok(())
    }

}


// TODO is there a better place for this stuff?
/// Moving to `mod process`
pub trait Resource {}

/// Moving to `mod <TBD>`
pub struct TaskList {
    head: Option<Box<Process>>,
}

/// Moving to `mod <TBD>`
pub struct TaskNode {
    proc: Option<Box<Process>>,
    prev: Option<Box<TaskNode>>,
    next: Option<Box<TaskNode>>,
}
