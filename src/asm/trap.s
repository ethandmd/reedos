### The current assuptions for context switching rely on these
### functions leaving the interrupt stack (sscratch stack) *exactly*
### as it was found, and not pushing or popping anything that remains
### / disappears after the trap exits


        .section .text
        ## This is the machine mode trap vector(not really). It exists
        ## to get us into the rust handler
        .option norvc
        .align 4
        .global __mtrapvec
__mtrapvec:
        csrrw sp, mscratch, sp
        save_gp_regs

        .extern m_handler
        call m_handler

        load_gp_regs
        csrrw sp, mscratch, sp
        mret

### ------------------------------------------------------------------
###
### Start of S mode stuff

        ## This is the supervisor trap handler
        .option norvc
        .align 4
        .globl __strapvec
__strapvec:
        csrrw sp, sscratch, sp
        sd t0, -8(sp)
        ## do early direction
        csrr t0, scause
        addi t0, t0, -8
        bnez t0, regular_strap
        ## Single out u mode scall
        ##
        ## I want to handle that seperately, reset state and move to
        ## handler
        ld t0, -8(sp)
        csrrw sp, sscratch, sp
        ## back to program stack
        j scall_asm

### handling a trap that was not a U mode syscall
###
### This is on the interrupt stack
regular_strap:
        ld t0, -8(sp)
        save_gp_regs

        ## load kernel page table
        ld t1, 264(sp)          #256 + 8

        li a0, 1
        sll a0, a0, 63
        ## top bit
        srl t1, t1, 12
        or t1, t1, a0
        ## top bit mode and PPN

        sfence.vma x0, x0
        csrw satp, t1
        sfence.vma x0, x0
        ## now in kernel space

        ## get gp back to restore more info from later
        ld gp, 256(sp)

        .extern s_handler
        call s_handler

        load_gp_regs
        csrrw sp, sscratch, sp
        sret


        ## The ecall / syscall handler is here.
        ##
        ## It follows the linux riscv calling convention for syscalls
        ##
        ## See
        ## https://stackoverflow.com/questions/59800430/risc-v-ecall-syscall-calling-convention-on-pk-linux
        ##
        ## This expects the call number in a7
        ## the arguments in a0-a5
        ## return value in a0
        ##
        ## The convention is that the caller saved registers are free
        ## to clobber as with a regular call
        ##
scall_asm:
        ## handle a yield specifically
        addi a7, a7, -124
        beqz a7, process_pause_asm
        ## This will finish making process safe to restore to, and get
        ## us back to the general kernel.

        ## otherwise just call a generic rust handler
        jal scall_rust

        ## TODO do we need to manually increment sepc? unclear
        sret
