#[no_mangle]
static mut SYS_RET: u64 = 0;

pub fn shuchu(ptr: usize, len: usize) {
    unsafe {
        core::arch::asm!(
            "mov r8, rsp",
            "lea r9, [rip + SYS_RET]",
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

pub fn rangchu() {
    unsafe {
        core::arch::asm!(
            "mov r8, rsp",
            "lea r9, [rip + SYS_RET]",
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

pub fn chuangjianduandian() -> u64 {
    unsafe {
        core::arch::asm!(
            "mov r8, rsp",
            "lea r9, [rip + SYS_RET]",
            "syscall",
            inout("rax") 3u64 => _,
            out("r8") _,
            out("r9") _,
            out("rcx") _,
            out("r11") _,
            options(nostack),
        );
        SYS_RET
    }
}

pub fn fasong(cap: u64, ptr: usize, len: usize) -> u64 {
    unsafe {
        core::arch::asm!(
            "mov r8, rsp",
            "lea r9, [rip + SYS_RET]",
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
        SYS_RET
    }
}

pub fn jieshou(cap: u64, ptr: usize, nonblock: u64) -> u64 {
    unsafe {
        core::arch::asm!(
            "mov r8, rsp",
            "lea r9, [rip + SYS_RET]",
            "syscall",
            inout("rax") 5u64 => _,
            in("rdi") cap,
            in("rsi") ptr,
            in("rdx") nonblock,
            out("r8") _,
            out("r9") _,
            out("rcx") _,
            out("r11") _,
            options(nostack),
        );
        SYS_RET
    }
}

pub fn duqupan(lba: u32, count: u32, buf: u64) -> u64 {
    unsafe {
        core::arch::asm!(
            "mov r8, rsp",
            "lea r9, [rip + SYS_RET]",
            "syscall",
            inout("rax") 9u64 => _,
            in("rdi") lba as u64,
            in("rsi") count as u64,
            in("rdx") buf,
            out("r8") _,
            out("r9") _,
            out("rcx") _,
            out("r11") _,
            options(nostack),
        );
        SYS_RET
    }
}

pub fn yingshe(vaddr: u64, phys: u64, pages: u64, flags: u64) {
    unsafe {
        core::arch::asm!(
            "mov r8, rsp",
            "lea r9, [rip + SYS_RET]",
            "syscall",
            inout("rax") 22u64 => _,
            in("rdi") vaddr,
            in("rsi") phys,
            in("rdx") pages,
            in("rcx") flags,
            out("r8") _,
            out("r9") _,
            out("r11") _,
            options(nostack),
        );
    }
}

pub fn jieyingshe(vaddr: u64, pages: u64) {
    unsafe {
        core::arch::asm!(
            "mov r8, rsp",
            "lea r9, [rip + SYS_RET]",
            "syscall",
            inout("rax") 23u64 => _,
            in("rdi") vaddr,
            in("rsi") pages,
            out("r8") _,
            out("r9") _,
            out("rcx") _,
            out("r11") _,
            options(nostack),
        );
    }
}

pub fn xiumian(ticks: u64) {
    unsafe {
        core::arch::asm!(
            "mov r8, rsp",
            "lea r9, [rip + SYS_RET]",
            "syscall",
            inout("rax") 24u64 => _,
            in("rdi") ticks,
            out("r8") _,
            out("r9") _,
            out("rcx") _,
            out("r11") _,
            options(nostack),
        );
    }
}

pub fn dakai(name: &[u8]) -> u32 {
    gvtcier_fat::fat_open(name)
}

pub fn duqu(handle: u32, buf: &mut [u8]) -> u32 {
    gvtcier_fat::fat_read(handle, buf)
}

pub fn guanbi(handle: u32) {
    gvtcier_fat::fat_close(handle);
}

pub fn xie(handle: u32, buf: &[u8]) -> u32 {
    gvtcier_fat::fat_write(handle, buf)
}

pub fn fenpei(order: u64) -> u64 {
    unsafe {
        core::arch::asm!(
            "mov r8, rsp",
            "lea r9, [rip + SYS_RET]",
            "syscall",
            inout("rax") 33u64 => _,
            in("rdi") order,
            out("r8") _,
            out("r9") _,
            out("rcx") _,
            out("r11") _,
            options(nostack),
        );
        SYS_RET
    }
}

pub fn shifang(index: u64, order: u64) {
    unsafe {
        core::arch::asm!(
            "mov r8, rsp",
            "lea r9, [rip + SYS_RET]",
            "syscall",
            inout("rax") 34u64 => _,
            in("rdi") index,
            in("rsi") order,
            out("r8") _,
            out("r9") _,
            out("rcx") _,
            out("r11") _,
            options(nostack),
        );
    }
}

pub fn yunxingtick() -> u64 {
    unsafe {
        core::arch::asm!(
            "mov r8, rsp",
            "lea r9, [rip + SYS_RET]",
            "syscall",
            inout("rax") 39u64 => _,
            out("r8") _,
            out("r9") _,
            out("rcx") _,
            out("r11") _,
            options(nostack),
        );
        SYS_RET
    }
}

pub fn liebian(caps_ptr: usize, len: usize) -> u64 {
    unsafe {
        core::arch::asm!(
            "mov r8, rsp",
            "lea r9, [rip + SYS_RET]",
            "syscall",
            inout("rax") 40u64 => _,
            in("rdi") caps_ptr as u64,
            in("rsi") len as u64,
            out("r8") _,
            out("r9") _,
            out("rcx") _,
            out("r11") _,
            options(nostack),
        );
        SYS_RET
    }
}
