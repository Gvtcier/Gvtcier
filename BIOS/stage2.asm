BITS 16
DEFAULT REL
org 0

%define FAT_START 65
%define ROOT_START 83
%define DATA_START 97

main_body:
    mov ax, cs
    mov ds, ax
    mov es, ax
    mov ss, ax
    mov sp, 0x7C00
    mov [drive], dl

    call serial_init
    mov si, msg_2
    call serial_print

    mov ax, 0x1000
    mov es, ax
    xor bx, bx
    mov si, ROOT_START
    mov cx, 14
read_root:
    push cx
    mov ax, si
    call read_sector
    jc disk_error
    pop cx
    inc si
    add bx, 512
    jnc root_no_adv
    mov ax, es
    add ax, 0x1000
    mov es, ax
root_no_adv:
    dec cx
    jnz read_root

    mov si, msg_r
    call serial_print

    mov si, 0
    mov cx, 224
find_loop:
    mov di, si
    push si
    push cx
    mov si, name_kernel
    mov cx, 11
    repe cmpsb
    pop cx
    pop si
    je found_kernel
    add si, 32
    loop find_loop
    mov si, msg_n
    call serial_print
    jmp disk_error

found_kernel:
    push si
    mov si, msg_k
    call serial_print
    pop si
    mov ax, es:[si + 26]
    mov [first_cluster], ax
    mov ax, es:[si + 28]
    mov [file_size], ax
    mov ax, es:[si + 30]
    mov [file_size + 2], ax

    mov ax, 0x1200
    mov es, ax
    xor bx, bx
    mov si, FAT_START
    mov cx, 18
read_fat:
    push cx
    mov ax, si
    call read_sector
    jc disk_error
    pop cx
    inc si
    add bx, 512
    jnc fat_no_adv
    mov ax, es
    add ax, 0x1000
    mov es, ax
fat_no_adv:
    dec cx
    jnz read_fat

    mov si, msg_f
    call serial_print

    mov ax, [first_cluster]
    mov [cur_cluster], ax
    mov word [load_seg], 0x2000
    mov word [load_bx], 0
read_cluster:
    mov ax, [cur_cluster]
    call print_hex16
    mov ax, [cur_cluster]
    sub ax, 2
    add ax, DATA_START
    push es
    mov ax, [load_seg]
    mov es, ax
    mov bx, [load_bx]
    mov ax, [cur_cluster]
    sub ax, 2
    add ax, DATA_START
    call read_sector
    jc disk_error
    pop es
    add word [load_bx], 512
    jnc no_seg_adv
    mov ax, [load_seg]
    add ax, 0x1000
    mov [load_seg], ax
no_seg_adv:
    mov ax, [cur_cluster]
    call fat12_next
    cmp ax, 0xFFF
    jae file_done
    mov [cur_cluster], ax
    jmp read_cluster

file_done:
    mov si, msg_c
    call serial_print

    mov ax, 0x1000
    mov es, ax
    xor di, di
    xor ebx, ebx
    xor bp, bp
e820_loop:
    mov eax, 0xE820
    mov edx, 0x534D4150
    mov ecx, 20
    int 0x15
    jc e820_done
    add di, 20
    inc bp
    test ebx, ebx
    jnz e820_loop
e820_done:
    cmp bp, 128
    jbe e820_clamped
    mov bp, 128
e820_clamped:
    mov [e820_count], bp

    cli
    in al, 0x92
    or al, 0x02
    out 0x92, al
    lgdt [gdt_desc]
    mov eax, cr0
    or eax, 1
    mov cr0, eax
    jmp dword 0x08:(pm32 + 0x18000)

BITS 32
pm32:
    mov ax, 0x10
    mov ds, ax
    mov es, ax
    mov ss, ax
    mov esp, 0x7C00
    mov ebx, 0x18000

    mov esi, 0x20000
    cmp dword [esi], 0x00786367
    jne pm_err
    mov eax, [esi + 12]
    mov [ebx + seg_num], eax
    mov eax, [esi + 16]
    mov [ebx + entry_lo], eax
    mov eax, [esi + 20]
    mov [ebx + entry_hi], eax
    mov eax, [esi + 24]
    mov [ebx + virt_base_lo], eax
    mov eax, [esi + 28]
    mov [ebx + virt_base_hi], eax
    mov eax, [esi + 32]
    mov [ebx + seg_off], eax

    mov ecx, [ebx + seg_num]
    mov ebp, [ebx + seg_off]
    add ebp, 0x20000
seg_loop:
    push ecx
    mov eax, [ebp + 8]
    mov edx, [ebp + 12]
    sub eax, [ebx + virt_base_lo]
    sbb edx, [ebx + virt_base_hi]
    mov edi, eax
    mov esi, [ebp]
    add esi, 0x20000
    mov ecx, [ebp + 16]
    shr ecx, 2
rep_copy:
    mov eax, [esi]
    mov [edi], eax
    add esi, 4
    add edi, 4
    dec ecx
    jnz rep_copy
    mov ecx, [ebp + 16]
    and ecx, 3
    jz no_remainder
rem_copy:
    mov al, [esi]
    mov [edi], al
    inc esi
    inc edi
    dec ecx
    jnz rem_copy
no_remainder:
    mov eax, [ebp + 24]
    sub eax, [ebp + 16]
    jz seg_done
    mov ecx, eax
    shr ecx, 2
    xor eax, eax
bss_zero:
    mov [edi], eax
    add edi, 4
    dec ecx
    jnz bss_zero
seg_done:
    pop ecx
    add ebp, 40
    dec ecx
    jnz seg_loop

    mov edi, 0x40000
    mov ecx, 28 * 1024 / 4
    xor eax, eax
zero_pt:
    mov [edi], eax
    add edi, 4
    dec ecx
    jnz zero_pt
    mov dword [0x40000], 0x41003
    mov dword [0x40004], 0
    mov dword [0x40000 + 256 * 8], 0x42003
    mov dword [0x40000 + 256 * 8 + 4], 0
    mov edi, 0x41000
    mov eax, 0x43003
    mov ecx, 4
pdp_lo:
    mov [edi], eax
    add edi, 8
    add eax, 0x1000
    dec ecx
    jnz pdp_lo
    mov edi, 0x42000
    mov eax, 0x43003
    mov ecx, 4
pdp_hi:
    mov [edi], eax
    add edi, 8
    add eax, 0x1000
    dec ecx
    jnz pdp_hi
    mov edi, 0x43000
    xor ebx, ebx
    mov ecx, 4
pd_page:
    push ecx
    mov ecx, 512
pd_item:
    mov eax, ebx
    or eax, 0x83
    mov [edi], eax
    add ebx, 0x200000
    add edi, 8
    dec ecx
    jnz pd_item
    pop ecx
    dec ecx
    jnz pd_page

    mov ebx, 0x18000

    mov dword [0x47000], 0x48000
    mov dword [0x47004], 0
    movzx eax, word [ebx + e820_count]
    mov dword [0x47008], eax
    mov dword [0x4700C], 0
    mov edi, 0x47010
    mov ecx, 24
zero_fb:
    mov byte [edi], 0
    inc edi
    dec ecx
    jnz zero_fb
    mov esi, 0x10000
    mov edi, 0x48000
    movzx ecx, word [ebx + e820_count]
mrs_loop:
    push ecx
    mov ebp, 1
    mov eax, [esi + 16]
    cmp eax, 1
    jne mrs_store
    mov eax, [esi + 4]
    test eax, eax
    jnz mrs_store
    mov eax, [esi]
    cmp eax, 0x1000000
    jae mrs_store
    mov ecx, 0x1000000
    sub ecx, eax
    mov eax, [esi + 8]
    cmp eax, ecx
    jbe mrs_skip
    sub eax, ecx
    mov [edi + 8], eax
    mov eax, [esi + 12]
    mov [edi + 12], eax
    mov dword [edi], 0x1000000
    mov dword [edi + 4], 0
    mov dword [edi + 16], 0
    jmp mrs_advance
mrs_skip:
    xor ebp, ebp
    jmp mrs_advance
mrs_store:
    mov eax, [esi]
    mov edx, [esi + 4]
    mov [edi], eax
    mov [edi + 4], edx
    mov eax, [esi + 8]
    mov edx, [esi + 12]
    mov [edi + 8], eax
    mov [edi + 12], edx
    mov eax, [esi + 16]
    cmp eax, 1
    jne kind_reserved
    xor eax, eax
    jmp kind_done
kind_reserved:
    mov eax, 1
kind_done:
    mov [edi + 16], eax
mrs_advance:
    add esi, 20
    test ebp, ebp
    jz mrs_no_adv
    add edi, 24
mrs_no_adv:
    pop ecx
    dec ecx
    jnz mrs_loop

    mov eax, edi
    sub eax, 0x48000
    mov ecx, 24
    xor edx, edx
    div ecx
    mov [0x47008], eax

    mov eax, cr4
    or eax, 0x20
    mov cr4, eax
    mov eax, 0x40000
    mov cr3, eax
    mov ecx, 0xC0000080
    rdmsr
    or eax, 0x100
    wrmsr
    mov eax, cr0
    or eax, 0x80000000
    mov cr0, eax
    jmp 0x18:(lm64 + 0x18000)

BITS 64
lm64:
    mov rdi, 0x47000
    mov rax, [rel entry_lo]
    mov rbx, [rel entry_hi]
    shl rbx, 32
    or rax, rbx
    jmp rax

BITS 16

disk_error:
    mov si, msg_err
    call serial_print
hang2:
    hlt
    jmp hang2

fat12_next:
    push bx
    push cx
    push dx
    mov bx, ax
    shr bx, 1
    add bx, ax
    push cs
    pop ds
    mov dx, 0x1200
    mov ds, dx
    mov dx, [bx]
    test ax, 1
    jz even_cluster
    shr dx, 4
    jmp got_next
even_cluster:
    and dx, 0x0FFF
got_next:
    mov ax, dx
    pop dx
    pop cx
    pop bx
    push cs
    pop ds
    ret

print:
    lodsb
    test al, al
    jz print_done
    mov ah, 0x0E
    int 0x10
    jmp print
print_done:
    ret

print_hex16:
    push ax
    push cx
    push dx
    mov cx, 4
ph_loop:
    rol ax, 4
    push ax
    and al, 0x0F
    cmp al, 10
    jb ph_num
    add al, 'A' - 10
    jmp ph_out
ph_num:
    add al, '0'
ph_out:
    call serial_putc
    pop ax
    dec cx
    jnz ph_loop
    pop dx
    pop cx
    pop ax
    ret

read_sector:
    push ax
    push si
    mov si, msg_x
    call serial_print
    pop si
    pop ax
    push cx
    push dx
    push bx
    xor dx, dx
    mov cx, 18
    div cx
    push ax
    mov cx, dx
    inc cx
    pop ax
    mov bx, ax
    shr bx, 1
    mov ch, bl
    and al, 1
    mov dh, al
    mov dl, [drive]
    mov ah, 0x02
    mov al, 1
    pop bx
    int 0x13
    push ax
    push si
    mov si, msg_y
    call serial_print
    pop si
    pop ax
    pop dx
    pop cx
    ret

serial_init:
    push ax
    push dx
    mov dx, 0x3F9
    xor al, al
    out dx, al
    mov dx, 0x3FB
    mov al, 0x80
    out dx, al
    mov dx, 0x3F8
    mov al, 1
    out dx, al
    mov dx, 0x3F9
    xor al, al
    out dx, al
    mov dx, 0x3FB
    mov al, 3
    out dx, al
    pop dx
    pop ax
    ret

serial_putc:
    push bx
    push dx
    mov bl, al
    mov dx, 0x3F8
    add dx, 5
wait_tx:
    in al, dx
    test al, 0x20
    jz wait_tx
    mov dx, 0x3F8
    mov al, bl
    out dx, al
    pop dx
    pop bx
    ret

serial_print:
    push ax
    push si
sp_loop:
    lodsb
    test al, al
    jz sp_done
    call serial_putc
    jmp sp_loop
sp_done:
    pop si
    pop ax
    ret

msg_2 db "2", 0
msg_r db "R", 0
msg_f db "F", 0
msg_c db "C", 0
msg_x db "x", 0
msg_y db "y", 0
msg_k db "K", 0
msg_n db "N", 0

name_kernel db "KERNEL  GCX"
msg_ok db "GCX loaded", 0
msg_err db "stage2 error", 0

BITS 16
gdt_start:
    dq 0
gdt_code32: dw 0xFFFF, 0, 0x9A00, 0x00CF
gdt_data:   dw 0xFFFF, 0, 0x9200, 0x00CF
gdt_code64: dw 0xFFFF, 0, 0x9A00, 0x00AF
gdt_end:
gdt_desc:
    dw gdt_end - gdt_start - 1
    dd gdt_start + 0x18000

BITS 32
pm_err:
    cli
    hlt
    jmp $

first_cluster dw 0
file_size dd 0
cur_cluster dw 0
lba dw 0
load_seg dw 0x2000
load_bx dw 0
e820_count dw 0
seg_num dd 0
entry_lo dd 0
entry_hi dd 0
virt_base_lo dd 0
virt_base_hi dd 0
seg_off dd 0
drive db 0
