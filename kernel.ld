OUTPUT_ARCH( "riscv" )
ENTRY( _entry )

MEMORY
{
  ram  (wxa) : ORIGIN = 0x80000000, LENGTH = 128M
}

/* If you don't Align(16), it breaks. If you try to page align .text,
it breaks.  Align seems to only work INSIDE sections. To make mapping
nice. Sections should be page aligned. */

SECTIONS
{
  .text : {
      *(.text .text.*)
      . = ALIGN(0x1000);
      PROVIDE(__text_end = .);
      PROVIDE(_etext = .);
  }

  PROVIDE(__global_pointer = .);

  .rodata : {
      *(.srodata .srodata.*)
      *(.rodata .rodata.*)
      . = ALIGN(0x1000);
      PROVIDE(_roedata = .);
  }
  .data : {
      *(.sdata .sdata.*)
      *(.data .data.*)
      . = ALIGN(0x1000);
      PROVIDE(_edata = .);
  }

  /* lower guard page included in above */
  .stacks : {
      . = ALIGN(0x1000);
      PROVIDE(__stacks_start = .);
      . = . + (4096 * 2 * 2); /* NHARTS with a guard page each, unstable */
      PROVIDE(__stacks_end = .);
  }
  . = . + 4096; /* guard page */
  /* stacks should start at stack end and alternate with guard pages going down */

  .bss : {
      . = ALIGN(0x1000);
      PROVIDE(__bss_start = .);
      *(.sbss .sbss.*)
      *(.bss .bss.*)
      . = ALIGN(0x1000);
      PROVIDE(__bss_end = .);
  }
  PROVIDE(_end = .);
  PROVIDE(__memory_end = ORIGIN(ram) + LENGTH(ram));
}
