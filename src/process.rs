//! Process handle and utilities.
// use alloc::boxed::Box;

// extern crate alloc;

// use alloc::boxed::Box;
use alloc::boxed::Box;
use alloc::collections::vec_deque::*;
use core::assert;
use core::mem::size_of;
use core::ptr::copy_nonoverlapping;
use core::arch::asm;

// use crate::hw::HartContext;
// use crate::trap::TrapFrame;
use crate::vm::ptable::*;
use crate::vm::{request_phys_page, PhysPageExtent};
use crate::file::elf64::*;

fn generate_new_pid() -> usize {
    todo!()
}

pub struct Process {
    saved_pc: usize,
    saved_sp: usize,
    id: usize,
    state: ProcessState,
    pgtbl: PageTable,
    phys_pages: VecDeque<PhysPageExtent>, // vec to avoid Ord requirement

    // currently unused, but needed in the future
    // address_space: BTreeSet<Box<dyn Resource>>, // todo: Balanced BST of Resources

    // unsused, possibly not needed ever
    // trapframe: TrapFrame,
    // ctx_regs: HartContext,
}

pub enum ProcessState {
    Uninitialized,              // do not attempt to run
    Unstarted,                  // do not attempt to restore regs
    Ready,                      // can run, restore args
    Running,                    // is running, don't use elsewhere
    // ^ is because ownership alone is risky to ensure safety accross
    // context switches
    Wait,                       // blocked on on something
    Sleep,                      // out of the running for a bit
    Dead,                       // do not run (needed?)
}

impl Process {
    pub fn new() -> Self {
        let pt = request_phys_page(1)
            .expect("Could not allocate a page table for a new process.");
        let mut out = Self {
            id: generate_new_pid(),
            state: ProcessState::Uninitialized,
            pgtbl: PageTable::new(pt.start()),
            phys_pages: VecDeque::default(),
            saved_pc: 0,
            saved_sp: 0,
        };
        out.phys_pages.push_back(pt);
        // ^ tranfers ownership of page table page to process struct
        out
    }

    pub fn initialize64(&mut self, elf: &ELFProgram) -> Result<(), ELFError> {
        // Doesn't check for uninitialized state so you can do a write over of an existing process
        match self.state {
            ProcessState::Running => {panic!("Tried to re-init a running process!")},
            _ => {},
        };
        self.populate_pagetable64(elf)?;
        self.saved_pc = elf.header.entry;
        self.state = ProcessState::Unstarted;
        Ok(())
    }

    /// Copies the LOAD segment memory layout from the elf to the
    /// program. This is not the only initialization step.
    fn populate_pagetable64(&mut self, elf: &ELFProgram) -> Result<(), ELFError>{
        assert!(elf.header.program_entry_size as usize == size_of::<ProgramHeaderSegment64>(),
        "Varying ELF entry size expectations.");

        let num = elf.header.num_program_entries;
        let ptr = elf.header.program_header_pos as *const ProgramHeaderSegment64;
        for i in 0..num {
            let segment = unsafe { *ptr.add(i as usize) };
            if segment.seg_type != ProgramSegmentType::Load { continue; }
            else if segment.vmem_addr < 0x1000 {return Err(ELFError::MappedZeroPage)}
            else if segment.size_in_file != segment.size_in_memory {return Err(ELFError::InequalSizes)}

            let n_pages = (segment.size_in_memory + (0x1000 - 1)) / 0x1000;
            let pages = match request_phys_page(n_pages as usize) {
                Ok(p) => {p},
                Err(_) => {return Err(ELFError::FailedAlloc)}
            };
            unsafe {
                copy_nonoverlapping(elf.source.add(segment.file_offset as usize),
                            pages.start() as *mut u8,
                            segment.size_in_file as usize);
            }
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


    /// This is a (kind of) context switch
    ///
    /// This intentionally does not consume the process, despite
    /// conceptually making it unavailable for other actors. That move
    /// out of a shared scope, if needed, should happen before this
    /// call.
    pub fn start(&mut self) -> ! {
        match self.state {
            ProcessState::Unstarted => {},
            _ => {panic!("Attempted to start an already started program!")},
        }
        self.state = ProcessState::Running;

        extern "C" {pub fn process_start_asm(pc: usize, pgtbl: usize) -> !;}

        unsafe {
            // we can't use PageTable.write_satp here becuase this is
            // not mapped into the process pagetable and it shouldn't
            // be. We want to do that later in the asm.
            // process_start_asm(self.saved_pc, self.pgtbl.base as usize);
            asm!("mv a0, {saved_pc}",
                 "mv a1, {base}",
                 ".extern process_start_asm",
                 "j process_start_asm",
                 saved_pc = in(reg) self.saved_pc,
                 base = in(reg) self.pgtbl.base);
        }
        panic!("Failed to jump into process!");
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
