MEMORY
{
  RAM : ORIGIN = 0x80000000, LENGTH = 1M
}

SECTIONS
{
  .text : {
    *(.text .text.*)
  } > RAM

  .rodata : {
    *(.rodata .rodata.*)
  } > RAM

  .data : {
    *(.data .data.*)
  } > RAM

  .bss : {
    *(.bss .bss.*)
  } > RAM

  PROVIDE(_end = .);
}
