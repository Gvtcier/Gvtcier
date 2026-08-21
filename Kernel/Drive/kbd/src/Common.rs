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
