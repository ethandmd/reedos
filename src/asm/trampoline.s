        ## This file contains the asm for context switches, the last
        ## thing that is run in kernel mode on a switch in, and the
        ## first thing on a switch out

        ## jump into a process that hasn't been run yet
        ## pc in a0, new base pagetable addr in a1, sp in a2
        ##
        ## we don't need to worry about saving registers, as this is a
        ## non-returning function call
        .global process_start_asm
process_start_asm:
        csrw sepc, a0
        ## return to the process on sret

        ## before swapping anything, we need to save the gp back to
        ## the top of the sscratch stack
        csrr a0, sscratch
        sd gp, (a0)

        mv sp, a2
        ## get onto the process stack, we will restore kernel stack
        ## with sscratch later

        li a0, 1
        sll a0, a0, 63
        ## top bit
        srl a1, a1, 12
        or a1, a1, a0
        ## top bit mode and PPN

        sfence.vma x0, x0
        csrw satp, a1
        sfence.vma x0, x0
        ## swap tables

        sret
        ## enter the process with usermode and pc/satp
        ##
        ## TODO do I need to worry about prior priviledge level not
        ## being U?

### ------------------------------------------------------------------

        ## jump into a process that has been run before
        ## takes pc in a0, new base pt in a1, and new sp in a2
        .global process_resume_asm
process_resume_asm:
        csrw sepc, a0
        ## return to the process on sret


        ## before we swap page tables, we need to save the gp info to
        ## a place we can restore to later
        ##
        ## Specifically the top of the sscratch stack
        csrr a0, sscratch
        sd gp, (a0)

        li a0, 1
        sll a0, a0, 63
        ## top bit
        srl a1, a1, 12
        or a1, a1, a0
        ## top bit mode and PPN

        sfence.vma x0, x0
        csrw satp, a1
        sfence.vma x0, x0
        ## swap tables

        mv sp, a2
        load_gp_regs
        sret
        ## jump there and enter U mode
        ## TODO worry about prior priv != U mode?

### ------------------------------------------------------------------
        ## this is more general code for switching stack + addr space
        ##
        ## These are for temporary excursions from process to kernel
        ## space, where there is a standard contiguous control flow
        ## between the start and end of the excursion

        .global proc_space_to_kernel_space
proc_space_to_kernel_space:
        ## we are on the process stack in the process space
        ##
        ## we need to get to kernel everything without clobbering the
        ## process stack or anything else. We don't need to worry
        ## about pc, because the calling convention handles that when
        ## we return after our kernel excursion. We return the process
        ## stack location in s3
        save_gp_regs            #caller saved regs

        mv s3, sp

        csrr sp, sscratch       #interrupt stack now
        ## sscratch stack holds, from low addr to high:
        ##
        ## the addr to restore to gp (see hartlocal.rs)
        ## the kernel page table (satp)
        ## the kernel stack (sp)

        ld t1, 8(sp)

        li t0, 1
        sll t0, t0, 63
        ## top bit
        srl t1, t1, 12
        or t1, t1, t0
        ## top bit mode and PPN

        sfence.vma x0, x0
        csrw satp, t1
        sfence.vma x0, x0

        ## in kernel space
        ld gp, (sp)
        ## further info restore
        ld sp, 16(sp)
        ## on kernel stack

        .extern kernel_excursion_rust
        j kernel_excursion_rust

### ------------------------------------------------------------------
        ## now the other direction, finishing the excursion. We get
        ## the process sp in a0, and the base page table in a1. The
        ## asm above should let us restore all the rest

        .global kernel_space_to_proc_space
kernel_space_to_proc_space:
        csrr sp, sscratch

        sd gp, (sp)             #make sure our new gpi is accessible

        li t0, 1
        sll t0, t0, 63
        ## top bit
        srl a1, a1, 12
        or a1, a1, t0
        ## top bit mode and PPN
        sfence.vma x0, x0
        csrw satp, a1
        sfence.vma x0, x0

        ## we are in process space

        mv sp, a0
        ## process stack

        load_gp_regs
        ret

### ------------------------------------------------------------------
        ## this is the end of the file
