# Kernel entry point.
# The linker script provides us with symbols which are necessary for
# us to configure our kernel memory layout. Primarily, notice that
# kernel code starts at address 0x80000000. Physical addresses below
# this contain memory mapped IO devices (CLINT, PLIC, UART NS16550A,
# ...).  This entry function is loaded at address 0x80000000 since
# it is a .text section and the linker lays those out first. This
# entry function's job is to set up the kernel stack so we have some
# space to work. Refer to src/param.rs for general memory
# layout. The kernel stack depends on the number of harts on the h/w
# (or qemu).  We mostly reference this from
# `xv6-riscv/kernel/entry.S`.

# For some reason gcc wants hashes but other people (like emacs) want to use semicolons

    .option norvc
    .section .text

    .global _entry
    _entry:

    .option push
    .option norelax
        # Linker position data relative to gp
    .extern __global_pointer
        la gp, __global_pointer
    .option pop
        # Set up stack per of hart ids according to linker script

        # Add 4k guard page per hart
        csrr a1, mhartid
        sll a1, a1, 1 # Multiple hartid by 2 to get alternating pages
        li a0, 0x1000
        mul a1, a1, a0
    .extern __stacks_end # Linker supplied
        la a2, __stacks_end
        sub sp, a2, a1

        # Jump to _start in src/main.rs
        .extern _start
        call _start
    spin:
        wfi
        j spin
