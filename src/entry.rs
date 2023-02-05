
// Learned about this use of global_asm! from
// https://dev-doc.rust-lang.org/beta/unstable-book/library-features/global-asm.html

pub mod entry {
    use core::arch::global_asm;
    global_asm!(
        ".section .text",
        ".global _entry",
        ".extern _start",
        "_entry:",
            "li sp, 0x90000000",
            "li a0, 1024*4",
            "csrr a1, mhartid",
            "addi a1, a1, 1",
            "mul a0, a0, a1",
            "add sp, sp, a0",
            // "# Jump to _start in src/main.rs",
            "call _start",
        "spin:",
            "wfi",
            "j spin",
    );
}
