OUTPUT_ARCH( "riscv" )
ENTRY( _entry )

MEMORY
{
  ram  (wxa) : ORIGIN = 0x80000000, LENGTH = 128M
}

SECTIONS
{
  .text : {
      *(.text .text.*)
      PROVIDE(__text_end = .);
  }
  
  PROVIDE(__global_pointer = .);
  
  .rodata : {
      *(.srodata .srodata.*)
      *(.rodata .rodata.*)
  } 
  .data : {
      *(.sdata .sdata.*)
      *(.data .data.*)
  }
  .bss : {
      *(.sbss .sbss.*)
      *(.bss .bss.*)
      . = ALIGN(0x1000);
  }
  PROVIDE(__bss_end = .);
  PROVIDE(__memory_end = ORIGIN(ram) + LENGTH(ram)); 
}
