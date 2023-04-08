        ## This file contains the asm for context switches, the last
        ## thing that is run in kernel mode on a switch in, and the
        ## first thing on a switch out

        ## The full contents of this file, and only this file, are
        ## mapped into the trampoline page
        ## .section .trampoline

        ## Not true, this is now part of text
        .section .text

        ## jump into a process that hasn't been run yet
        ## pc in a0, new base pagetable addr in a1
        ##
        ## we don't need to worry about saving registers, as this is a
        ## non-returning function call
        .global process_start_asm
process_start_asm:
        csrw sepc, a0
        sfence.vma x0, x0
        csrw satp, a1
        sfence.vma x0, x0
        sret
        ## current idea is to dip into M mode so that we can write
        ## satp and flush the TLB, and then jump into the program with
        ## a mret into user mode w/ mepc as the program entry

        ## this is the end of the file