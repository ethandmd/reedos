# This should be at 0x80000000 from kernel.ld linker script.
# We use the section .textstart here that we defined in kernel.ld

#No .data section
.section .textstart
.global _entry
_entry:
	# Reference chapter 25 for RISC-V asm handbook with register conventions.
	# 'CSR': Control and Status Register (Chapter 9 of riscv spec).

	# Here we force no relax for gp.
	# https://sourceware.org/binutils/docs-2.31/as/RISC_002dV_002dDirectives.html#RISC_002dV_002dDirectives
.option push
.option norelax
	la	gp, __global_pointer
.option pop

	la	sp, __stack_top
