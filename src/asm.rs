use core::arch::global_asm;

global_asm!(include_str!("asm/entry.s"));
global_asm!(include_str!("asm/trap.s"));
