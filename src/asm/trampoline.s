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

        ## jump into after a process calls yield
        ##
        ## Takes no args, save state to restore later, and get us back
        ## to the main part of the kernel
        ##
        ## This should execute in S mode
process_pause_asm:
        save_gp_regs
        ## onto PROCESS stack

        ## hold onto what we need to save
        csrr a0, sepc
        mv a1, sp

        ## sscratch holds the interrupt stack
        csrr sp, sscratch

        ## sscratch stack holds, from low addr to high:
        ##
        ## the addr to restore to gp (see hartlocal.rs)
        ## the kernel page table (satp)
        ## the kernel stack (sp)

        ## load kernel page table
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

        ## get gp back to restore more info from later
        ld gp, (sp)
        ## get on the main kernel stack
        ld sp, 16(sp)

        ## Load the cause of the process pause, 0 for none
        li a2, 0

        ## we want to save the changed registers to restore later
        .extern process_pause_rust
        j process_pause_rust

### ------------------------------------------------------------------

        ## this is the end of the file
