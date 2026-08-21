use core::arch::global_asm;
use core::panic::PanicInfo;

#[panic_handler]
pub fn panic(_info: &PanicInfo) -> ! {
    loop {
        unsafe { core::arch::asm!("hlt", options(nomem, nostack)) }
    }
}

#[no_mangle]
static mut RET_SLOT: u64 = 0;

global_asm!(
    ".global _start",
    "_start:",
    "mov rsp, 0x8000000",
    "call user_main",
    ".Ldead:",
    "pause",
    "jmp .Ldead",
);

pub fn sys_write(ptr: usize, len: usize) {
    unsafe {
        core::arch::asm!(
            "mov r8, rsp",
            "lea r9, [rip + RET_SLOT]",
            "syscall",
            inout("rax") 1u64 => _,
            in("rdi") ptr,
            in("rsi") len,
            out("r8") _,
            out("r9") _,
            out("rcx") _,
            out("r11") _,
            options(nostack),
        );
    }
}

pub fn sys_yield() {
    unsafe {
        core::arch::asm!(
            "mov r8, rsp",
            "lea r9, [rip + RET_SLOT]",
            "syscall",
            inout("rax") 2u64 => _,
            out("r8") _,
            out("r9") _,
            out("rcx") _,
            out("r11") _,
            options(nostack),
        );
    }
}

pub fn sys_ep_create() -> u64 {
    unsafe {
        core::arch::asm!(
            "mov r8, rsp",
            "lea r9, [rip + RET_SLOT]",
            "syscall",
            inout("rax") 3u64 => _,
            out("r8") _,
            out("r9") _,
            out("rcx") _,
            out("r11") _,
            options(nostack),
        );
        RET_SLOT
    }
}

pub fn sys_send(cap: u64, ptr: usize, len: usize) -> u64 {
    unsafe {
        core::arch::asm!(
            "mov r8, rsp",
            "lea r9, [rip + RET_SLOT]",
            "syscall",
            inout("rax") 4u64 => _,
            in("rdi") cap,
            in("rsi") ptr,
            in("rdx") len,
            out("r8") _,
            out("r9") _,
            out("rcx") _,
            out("r11") _,
            options(nostack),
        );
        RET_SLOT
    }
}

pub fn sys_recv(cap: u64, ptr: usize) -> u64 {
    unsafe {
        core::arch::asm!(
            "mov r8, rsp",
            "lea r9, [rip + RET_SLOT]",
            "syscall",
            inout("rax") 5u64 => _,
            in("rdi") cap,
            in("rsi") ptr,
            out("r8") _,
            out("r9") _,
            out("rcx") _,
            out("r11") _,
            options(nostack),
        );
        RET_SLOT
    }
}

pub fn sys_dbg() -> u64 {
    unsafe {
        core::arch::asm!(
            "mov r8, rsp",
            "lea r9, [rip + RET_SLOT]",
            "syscall",
            inout("rax") 6u64 => _,
            out("r8") _,
            out("r9") _,
            out("rcx") _,
            out("r11") _,
            options(nostack),
        );
        RET_SLOT
    }
}

pub fn g2d_canvas_create(w: u64, h: u64, buf: u64) -> u64 {
    unsafe {
        core::arch::asm!(
            "mov r8, rsp",
            "lea r9, [rip + RET_SLOT]",
            "syscall",
            inout("rax") 13u64 => _,
            in("rdi") w,
            in("rsi") h,
            in("rdx") buf,
            out("r8") _,
            out("r9") _,
            out("rcx") _,
            out("r11") _,
            options(nostack),
        );
        RET_SLOT
    }
}

pub fn g2d_canvas_map(id: u64) {
    unsafe {
        core::arch::asm!(
            "mov r8, rsp",
            "lea r9, [rip + RET_SLOT]",
            "syscall",
            inout("rax") 15u64 => _,
            in("rdi") id,
            out("r8") _,
            out("r9") _,
            out("rcx") _,
            out("r11") _,
            options(nostack),
        );
    }
}

pub fn g2d_compose(id: u64, x: u64, y: u64) {
    unsafe {
        core::arch::asm!(
            "mov r8, rsp",
            "lea r9, [rip + RET_SLOT]",
            "syscall",
            inout("rax") 16u64 => _,
            in("rdi") id,
            in("rsi") x,
            in("rdx") y,
            out("r8") _,
            out("r9") _,
            out("rcx") _,
            out("r11") _,
            options(nostack),
        );
    }
}

pub fn g2d_flush() {
    unsafe {
        core::arch::asm!(
            "mov r8, rsp",
            "lea r9, [rip + RET_SLOT]",
            "syscall",
            inout("rax") 17u64 => _,
            out("r8") _,
            out("r9") _,
            out("rcx") _,
            out("r11") _,
            options(nostack),
        );
    }
}

pub fn g2d_fill(id: u64, w: u64, h: u64, color: u32) {
    unsafe {
        core::arch::asm!(
            "mov r8, rsp",
            "syscall",
            inout("rax") 25u64 => _,
            in("rdi") id,
            in("rsi") w,
            in("rdx") h,
            in("r10") color as u64,
            out("r8") _,
            out("r11") _,
            options(nostack),
        );
    }
}

pub fn g2d_rect(id: u64, x: u64, y: u64, rw: u64, rh: u64, color: u32) {
    unsafe {
        core::arch::asm!(
            "mov r8, rsp",
            "push {c}",
            "syscall",
            inout("rax") 26u64 => _,
            in("rdi") id,
            in("rsi") x,
            in("rdx") y,
            in("r10") rw,
            in("r9") rh,
            c = in(reg) color as u64,
            out("r8") _,
            out("r11") _,
            options(nostack),
        );
    }
}

pub fn g2d_line(id: u64, x1: u64, y1: u64, x2: u64, y2: u64, color: u32) {
    unsafe {
        core::arch::asm!(
            "mov r8, rsp",
            "push {c}",
            "syscall",
            inout("rax") 27u64 => _,
            in("rdi") id,
            in("rsi") x1,
            in("rdx") y1,
            in("r10") x2,
            in("r9") y2,
            c = in(reg) color as u64,
            out("r8") _,
            out("r11") _,
            options(nostack),
        );
    }
}

pub fn g2d_char(id: u64, x: u64, y: u64, ch: u8, color: u32) {
    unsafe {
        core::arch::asm!(
            "mov r8, rsp",
            "syscall",
            inout("rax") 28u64 => _,
            in("rdi") id,
            in("rsi") x,
            in("rdx") y,
            in("r10") ch as u64,
            in("r9") color as u64,
            out("r8") _,
            out("r11") _,
            options(nostack),
        );
    }
}

pub fn g2d_text(id: u64, x: u64, y: u64, color: u32, text: &[u8]) {
    unsafe {
        core::arch::asm!(
            "mov r8, rsp",
            "syscall",
            inout("rax") 29u64 => _,
            in("rdi") id,
            in("rsi") x,
            in("rdx") y,
            in("r10") color as u64,
            in("r9") text.as_ptr() as u64,
            out("r8") _,
            out("r11") _,
            options(nostack),
        );
    }
}

pub fn g2d_curve(id: u64, pts: &[(usize, usize)], color: u32, seg: u64) {
    unsafe {
        core::arch::asm!(
            "mov r8, rsp",
            "syscall",
            inout("rax") 35u64 => _,
            in("rdi") id,
            in("rsi") pts.as_ptr() as u64,
            in("rdx") color as u64,
            in("r10") seg,
            out("r8") _,
            out("r11") _,
            options(nostack),
        );
    }
}

pub fn print_hex(v: u64) {
    let mut buf = [0u8; 32];
    buf[0] = b'0';
    buf[1] = b'x';
    let mut o = 2;
    for i in (0..16).rev() {
        let d = ((v >> (i * 4)) & 0xF) as u8;
        buf[o] = if d < 10 { b'0' + d } else { b'a' + d - 10 };
        o += 1;
    }
    sys_write(buf.as_ptr() as usize, 18);
}

pub fn print(s: &str) {
    sys_write(s.as_ptr() as usize, s.len());
}
