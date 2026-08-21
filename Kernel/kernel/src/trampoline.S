[bits 16]
section .text
global ap_trampoline
global ap_trampoline_end
global ap_cr3
global ap_stack_top

ap_trampoline:
    cli
    xor ax, ax
    mov ds, ax
    mov es, ax
    mov ss, ax
    lgdt [0x8FF8]
    mov eax, cr0
    or eax, 1
    mov cr0, eax
    jmp 0x08:prot_mode - ap_trampoline + 0x8000

[bits 32]
prot_mode:
    mov eax, 0x10
    mov ds, eax
    mov es, eax
    mov ss, eax
    mov eax, cr4
    or eax, 0x20
    mov cr4, eax
    mov ecx, 0xC0000080
    rdmsr
    or eax, 0x100
    wrmsr
    mov eax, cr0
    or eax, 0x80000000
    mov cr0, eax
    jmp 0x18:long_mode - ap_trampoline + 0x8000

[bits 64]
long_mode:
    mov rax, [rel ap_cr3]
    mov cr3, rax
    mov rax, [rel ap_stack_top]
    mov rsp, rax
    extern ap_entry
    mov rax, ap_entry
    jmp rax

align 8
ap_cr3:
    dq 0
ap_stack_top:
    dq 0
ap_trampoline_end:
