ENTRY(_start)

SECTIONS
{
    . = 0x400000;
    .text : { *(.text*) }
    .rodata : { *(.rodata*) }
    .data : { *(.data*) }
    .bss : {
        *(.bss*) *(COMMON)
    }
}

_user_stack_top = 0x804000;
