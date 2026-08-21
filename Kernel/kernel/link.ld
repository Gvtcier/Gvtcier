ENTRY(_start)

SECTIONS
{
    . = 0xFFFF800000000000 + 0x200000;
    .text : { *(.text*) }
    .rodata : { *(.rodata*) }
    .data : { *(.data*) }
    .bss : {
        *(.bss*) *(COMMON)
        . = ALIGN(16);
        . = . + 0x4000;
        _stack_top = .;
    }
}
