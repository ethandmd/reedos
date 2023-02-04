/* 
 * This is a minimal linker script, designed to get us up and running
 * with some "mellow swirled!" action. (a.k.a. get us to the bootloader).
 *
 * Also, any byte alignment will be labeled in n := # of bytes, not hex.
 * e.g. ALIGN(4096), for 4k pages, if we want to be page aligned.
 *
 * References:
 *  riscv-...-ld linker script from gnu riscv toolchain.
 *  siFive linker script generator: https://github.com/sifive/ldscript-generator
 *  riscv-isa-manual: https://github.com/riscv/riscv-isa-manual
 *  rust-embedded riscv-rt: https://github.com/rust-embedded/riscv-rt/blob/master/link.x
 *  xv6-riscv: https://github.com/mit-pdos/xv6-riscv/blob/riscv/kernel/kernel.ld
 */

OUTPUT_ARCH("riscv")
/* Set the entry point symbol called '_start'. */
ENTRY(_start)

MEMORY
{
    ram (wxa) : ORIGIN = 0x80000000, LENGTH = 128M
}

/*
 * .text = executable sections
 * .rodata = global constants
 * .data = global initialized variables
 * .bss = global uninitialized variables
 */

SECTIONS
{
    /*
     * This is where qemu's '-kernel' jumps to. Ensure _entry is at this address.
     * The period '.' := current memory location.
     * Check qemu memory map:
     *     .../qemu/hw/riscv/virt.c
     */
    . = 0x80000000;

    .text : {
        /*
         * First part of RAM layout is .text (see above), so we need this section
         * to line up our entry point with 0x80000000.
         * 
         * This lays out text sections first, so let's do ourselves a favor and add
         * in a '.textstart' to make super sure it comes first in the layout.
         */
        *(.textstart)
        *(.text .text.*)
    }

    /* Just for relative positioning for below sections. */
    PROVIDE(__global_pointer = .);

    /* Don't worry about srodata vs rodata. Let the compiler care. */
    .rodata : {
        *(.srodata .srodata.*)
        *(.rodata .rodata.*)
    }

    .data : {
        /* For data section, we probably want to align to our page size, 4Kb. */
        . = ALIGN(4096)
        *(.sdata .sdata.*)
        *(.data .data.*)
    }

    .bss : {
        *(.sbss .sbss.*)
        *(.bss .bss.*)
    }

    /* These will be useful symbols. */
    PROVIDE(__stack_top = ORIGIN(ram) + LENGTH(ram));
}
