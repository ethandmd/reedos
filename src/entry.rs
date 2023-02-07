// Learned about this use of global_asm! from
// https://dev-doc.rust-lang.org/beta/unstable-book/library-features/global-asm.html
use core::arch::global_asm;

global_asm!(r#"
    .section .text
    .global _entry
    .extern _start
    _entry:
        # Bootstrap on hartid 0
        csrr t0, mhartid
        bne t0, x0, spin
    
    # Riscv relax, look it up. No good for gp addr.
    .option push
    .option norelax
        # Linker position data relative to gp
        la gp, _global_pointer
    .option pop

        # Set up stack
        li t0, 0x0
        li t0, 0x1000 # = 4096
        li t1, 0x2 # For param::NHART == 2...this is unstable.
        mul t0, t0, t1 # 4096 * NHART
        la sp, end  # end -> end of .bss
        add sp, sp, t0 # Setup stack ptr at offset + end of .bss

        # Add 4k guard page per hart
        li a0, 0x1000
        csrr a1, mhartid
        addi a1, a1, 1
        mul a0, a0, a1
        add sp, sp, a0
        
        # Jump to _start in src/main.rs
        call _start
    spin:
        wfi
        j spin
"#);
