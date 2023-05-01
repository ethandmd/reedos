/// This module is just to clean up the process module so it is
/// actually readable without scrolling past all this stuff

use core::mem::swap;

use super::*;

impl Process {
    /// Construct a new process. Notably does not allocate anything or
    /// mean anything until you initialize it.
    pub fn new_uninit() -> Self {
        let out = Self {
            id: 0,
            state: ProcessState::Uninitialized,
            pgtbl: PageTable::new(null_mut()),
            phys_pages: MaybeUninit::uninit(),
            resource_id_gen: IdGenerator::new(),
            held_resources: MaybeUninit::uninit(),
            saved_pc: 0,
            saved_sp: 0,
        };
        out
    }

    pub fn initialize64(&mut self, elf: &ELFProgram) -> Result<(), ELFError> {
        // Doesn't assert uninitialized state so you can do a write over of an existing process
        match self.state {
            ProcessState::Uninitialized => {
                self.id = generate_new_pid();
                let pt = request_phys_page(1)
                    .expect("Could not allocate a page table for a new process.");
                self.pgtbl = PageTable::new(pt.start());
                self.phys_pages.write(VecDeque::new());
                self.held_resources.write(ResourceList {
                    contents: BTreeMap::new()
                });
                unsafe {
                    self.phys_pages.assume_init_mut().push_back(pt);
                }
            },
            ProcessState::Running => {
                panic!("Tried to re-initialize a running process!");
            },
            ProcessState::Blocked(_) => {
                panic!("Tried to re-init while waiting for something!")
            },
            _ => {
                unsafe {
                    let mut old_resources: ResourceList = ResourceList {contents: BTreeMap::new()};
                    swap(&mut old_resources, self.held_resources.assume_init_mut());
                    for (_id, res) in old_resources.contents.into_iter() {
                        res.pair.1.release(res.pair.0)
                    }
                }
            },
        }


        self.populate_pagetable64(elf)?;
        match self.map_kernel_text() {
            Ok(_) => {},
            Err(_) => {
                panic!("Failed to map kernel text into process space!");
            }
        }
        self.saved_pc = elf.header.entry;
        self.state = ProcessState::Unstarted;
        Ok(())
    }

    // TODO is this the right error type?
    fn map_kernel_text(&mut self) -> Result<(), VmError> {
        // This is currently a large copy of kpage_init with a few tweaks
        page_map(
            self.pgtbl,
            text_start(),
            text_start(),
            text_end().addr() - text_start().addr(),
            kernel_process_flags(true, false, true)
        )?;
        // log!(
        //     Debug,
        //     "Sucessfully mapped kernel text into process pgtable..."
        // );

        page_map(
            self.pgtbl,
            text_end(),
            text_end() as *mut usize,
            rodata_end().addr() - text_end().addr(),
            kernel_process_flags(true, false, false),
        )?;
        // log!(
        //     Debug,
        //     "Succesfully mapped kernel rodata into process pgtable..."
        // );

        page_map(
            self.pgtbl,
            rodata_end(),
            rodata_end() as *mut usize,
            data_end().addr() - rodata_end().addr(),
            kernel_process_flags(true, true, false),
        )?;
        // log!(
        //     Debug,
        //     "Succesfully mapped kernel data into process pgtable..."
        // );

        // This maps hart 0, 1 stack pages in opposite order as entry.S. Shouln't necessarily be a
        // problem.
        let base = stacks_start();
        for s in 0..NHART {
            let stack = unsafe { base.byte_add(PAGE_SIZE * (1 + s * 3)) };
            page_map(
                self.pgtbl,
                stack,
                stack,
                PAGE_SIZE * 2,
                kernel_process_flags(true, true, false),
            )?;
        //     log!(
        //         Debug,
        //         "Succesfully mapped kernel stack {} into process pgtable...",
        //         s
        //     );
        }

        // This maps hart 0, 1 stack pages in opposite order as entry.S. Shouln't necessarily be a
        // problem.
        let base = intstacks_start();
        for i in 0..NHART {
            let m_intstack = unsafe { base.byte_add(PAGE_SIZE * (1 + i * 4)) };
            // Map hart i m-mode handler.
            page_map(
                self.pgtbl,
                m_intstack,
                m_intstack,
                PAGE_SIZE,
                kernel_process_flags(true, true, false),
            )?;
            // Map hart i s-mode handler
            let s_intstack = unsafe { m_intstack.byte_add(PAGE_SIZE * 2) };
            page_map(
                self.pgtbl,
                s_intstack,
                s_intstack,
                PAGE_SIZE,
                kernel_process_flags(true, true, false),
            )?;
            // log!(
            //     Debug,
            //     "Succesfully mapped interrupt stack for hart {} into process pgtable...",
            //     i
            // );
        }

        page_map(
            self.pgtbl,
            bss_start(),
            bss_start(),
            bss_end().addr() - bss_start().addr(),
            kernel_process_flags(true, true, false),
        )?;
        // log!(Debug, "Succesfully mapped kernel bss into process...");

        page_map(
            self.pgtbl,
            bss_end(),
            bss_end(),
            memory_end().addr() - bss_end().addr(),
            kernel_process_flags(true, true, false),
        )?;
        // log!(Debug, "Succesfully mapped kernel heap into process...");

        Ok(())
    }


    // TODO better error type here?
    /// Copies the LOAD segment memory layout from the elf to the
    /// program. This is not the only initialization step.
    ///
    /// This also setups up the program stack and sets saved_sp
    fn populate_pagetable64(&mut self, elf: &ELFProgram) -> Result<(), ELFError>{
        assert!(elf.header.program_entry_size as usize == size_of::<ProgramHeaderSegment64>(),
                "Varying ELF entry size expectations.");

        let num = elf.header.num_program_entries;
        let ptr = unsafe {
            elf.source.add(elf.header.program_header_pos)
                as *const ProgramHeaderSegment64
        };
        for i in 0..num {
            let segment = unsafe { *ptr.add(i as usize) };
            if segment.seg_type != ProgramSegmentType::Load { continue; }
            else if segment.vmem_addr < 0x1000  { return Err(ELFError::MappedZeroPage) }
            else if segment.vmem_addr >= text_start().addr() as u64 &&
                segment.vmem_addr <= text_end().addr() as u64 {
                    return Err(ELFError::MappedKernelText)
                }
            else if segment.size_in_file != segment.size_in_memory {return Err(ELFError::InequalSizes)}
            else if segment.alignment > 0x1000 {return Err(ELFError::ExcessiveAlignment)}

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
                (segment.flags as u16) & PROG_SEG_READ != 0,
                (segment.flags as u16) & PROG_SEG_WRITE != 0,
                (segment.flags as u16) & PROG_SEG_EXEC != 0
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
            unsafe {
                self.phys_pages.assume_init_mut().push_back(pages);
            }
        }

        // TODO what does process heap look like? depends on our syscalls I guess?
        // We would map it here if we had any

        // map the process stack. They will get 2 pages for now
        const STACK_PAGES: usize = 2;
        let stack_pages = match request_phys_page(STACK_PAGES) {
            Ok(p) => {p},
            Err(_) => {
                return Err(ELFError::FailedAlloc);
            }
        };
        // TODO guard page? you'll get a page fault anyway?
        let process_stack_location = unsafe {
            text_start().sub(0x1000 * STACK_PAGES)
        };
        // under the kernel text
        match page_map(
            self.pgtbl,
            VirtAddress::from(process_stack_location),
            PhysAddress::from(stack_pages.start()),
            STACK_PAGES,
            user_process_flags(true, true, false)
        ) {
            Ok(_) =>{},
            Err(_) => {return Err(ELFError::FailedMap)}
        }
        self.saved_sp = stack_pages.end() as usize;
        unsafe {
            self.phys_pages.assume_init_mut().push_back(stack_pages);
        }

        Ok(())
    }

}
