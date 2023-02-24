OUTPUT_ARCH( "riscv" )
ENTRY( _entry )

MEMORY
{
  ram  (wxa) : ORIGIN = 0x80000000, LENGTH = 128M
}

/* If you don't Align(16), it breaks. If you try to page align .text, it breaks. */

SECTIONS
{
  .text : {
      *(.text .text.*)
      /*. = ALIGN(0x1000);*/
      PROVIDE(__text_end = .);
  }

  PROVIDE(__global_pointer = .);

  .rodata : {
      . = ALIGN(16);
      *(.srodata .srodata.*)
      *(.rodata .rodata.*)
      . = ALIGN(16);
  }
  .data : {
      . = ALIGN(16);
      *(.sdata .sdata.*)
      *(.data .data.*)
      . = ALIGN(16);
  }
  .bss : {
      . = ALIGN(16);
      *(.sbss .sbss.*)
      *(.bss .bss.*)
      . = ALIGN(16);
  }
  . = ALIGN(0x1000);
  PROVIDE(__bss_end = .);
  PROVIDE(__memory_end = ORIGIN(ram) + LENGTH(ram));
}
