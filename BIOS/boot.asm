BITS 16
org 0x7C00

jmp short start
nop
times 0x3E-($-$$) db 0

start:
    cli
    xor ax, ax
    mov ds, ax
    mov es, ax
    mov ss, ax
    mov sp, 0x7C00
    sti
    mov [drive], dl

    call serial_init
    mov si, msg_b
    call serial_print

    mov ax, 0x1800
    mov es, ax
    xor bx, bx
    mov ah, 0x02
    mov al, 64
    mov ch, 0
    mov cl, 2
    mov dh, 0
    mov dl, [drive]
    int 0x13
    jc disk_error

    mov si, msg_s
    call serial_print

    jmp 0x1800:0x0000

disk_error:
    mov si, msg_err
    call serial_print
hang:
    hlt
    jmp hang

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

msg_b db "B", 0
msg_s db "S", 0
msg_err db "BOOT ERR", 0
drive db 0

times 510-($-$$) db 0
dw 0xAA55
