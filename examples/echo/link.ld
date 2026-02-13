ENTRY(_start)

MEMORY
{
  USER_MEM (rwx) : ORIGIN = 0x10000, LENGTH = 1M
}

SECTIONS
{
  .text : {
    *(.text .text.*)
  } > USER_MEM

  .rodata : {
    *(.rodata .rodata.*)
    *(.srodata .srodata.*)
  } > USER_MEM

  .data : {
    *(.data .data.*)
    *(.sdata .sdata.*)
  } > USER_MEM

  .bss : {
    *(.bss .bss.*)
    *(.sbss .sbss.*)
  } > USER_MEM

  /DISCARD/ : {
    *(.eh_frame)
  }
}
