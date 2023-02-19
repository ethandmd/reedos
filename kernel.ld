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
      . = ALIGN(0x1000);
  } /* >ram AT>ram :text */
  
  PROVIDE(_global_pointer = .);
  
  .rodata : {
      *(.srodata .srodata.*)
      *(.rodata .rodata.*)
  } /* >ram AT>ram :text */

  .data : {
      . = ALIGN(4096);
      *(.sdata .sdata.*)
      *(.data .data.*)
  } /* >ram AT>ram :data */

  .bss : {
      *(.sbss .sbss.*)
      *(.bss .bss.*)
  
  } /* >ram AT>ram :bss */

  PROVIDE(__bss_end = .);
  PROVIDE(__memory_end = ORIGIN(ram) + LENGTH(ram)); 
}
