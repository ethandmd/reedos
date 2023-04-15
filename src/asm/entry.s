### Kernel entry point.
### The linker script provides us with symbols which are necessary for us to
### configure our kernel memory layout. Primarily, notice that kernel code starts
### at address 0x80000000. Physical addresses below this contain memory mapped IO
### devices (CLINT, PLIC, UART NS16550A, ...).  This entry function is loaded at
### address 0x80000000 since it is a .text section and the linker lays those out
### first. This entry function's job is to set up the kernel stack so we have some
### space to work. Refer to src/param.rs for general memory layout. The kernel
### stack depends on the number of harts on the h/w (or qemu).  We mostly
### reference this from `xv6-riscv/kernel/entry.S`.

        .option norvc
        .section .text.entry
        .global _entry
_entry:
        .option push
        .option norelax
        ## Linker position data relative to gp
        ## See linkerscript for details on why we don't need this
        ## .extern _global_pointer
        ## la gp, _global_pointer
        ##
        ## Instead we are using gp while in the kernel to point to the
        ## hart local data, which for now will be the process they are
        ## running (or an uninitialized one)

        .option pop
        ## Set up stack per of hart ids according to linker script

        ## Add 4k guard page per hart
        csrr a1, mhartid
        li a0, 0x3000           #2 page stack + guard page
        mul a1, a1, a0          #offset by hart id
        .extern _stacks_end
        la a2, _stacks_end      # this is the top byte for hart 0
        sub sp, a2, a1

        .extern _intstacks_end
        csrr a1, mhartid
        li a0, 0x4000
        mul a1, a1, a0
        la a2, _intstacks_end
        sub a2, a2, a1
        csrw mscratch, a2 # Write per hart mscratch pad


        li a0, 0x2000
        sub a2, a2, a0 # Move sp down by scratch pad page + guard page

        ## put half of the initial contents of the sscratch stack in
        ## now, namely the kernel stack base addr for this hart
        addi a2, a2, -8
        sd sp, (a2)
        csrw sscratch, a2 # Write per hart sscratch pad

                                # Jump to _start in src/main.rs
        .extern _start
        call _start
spin:
        wfi
        j spin
