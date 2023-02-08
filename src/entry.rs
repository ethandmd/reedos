//! Kernel entry point.
/// The linker script provides us with symbols which are necessary
/// for us to configure our kernel memory layout. Primarily, notice that
/// kernel code starts at address 0x80000000. Physical addresses below this
/// contain memory mapped IO devices (CLINT, PLIC, UART NS16550A, ...). 
/// This entry function is loaded at address 0x80000000 since it is a .text section
/// and the linker lays those out first. This entry function's job is to set up the
/// kernel stack so we have some space to work. Refer to src/param.rs for general
/// memory layout. The kernel stack depends on the number of harts on the h/w (or qemu).
/// We mostly reference this from `xv6-riscv/kernel/entry.S` and follow their memory layout.
/// TODO: We have not yet implemented the trampoline mechanism.
/// But notice the use of inline `global_asm!`, and that the `_start` function is 
/// visible to this script. 
// Learned about this use of global_asm! from
// https://dev-doc.rust-lang.org/beta/unstable-book/library-features/global-asm.html
use core::arch::global_asm;

global_asm!(
    r#"
    .section .text
    .global _entry
    .extern _start
    _entry:
    # Riscv relax, look it up. No good for gp addr.
    .option push
    .option norelax
        # Linker position data relative to gp
        la gp, _global_pointer
    .option pop

        # Set up stack per # of hart ids
        li t0, 0x0
        li t0, 0x1000 # = 4096
        li t1, 0x2 # For param::NHART == 2...this is unstable.
        mulw t0, t0, t1 # 4096 * NHART
        la sp, end
        add sp, sp, t0 # Setup stack ptr at offset + end of .bss

        # Add 4k guard page per hart
        li a0, 0x1000
        csrr a1, mhartid
        addi a1, a1, 1
        mulw a0, a0, a1
        add sp, sp, a0
        
        # Jump to _start in src/main.rs
        call _start
    spin:
        # wfi
        j spin
    "#
);
