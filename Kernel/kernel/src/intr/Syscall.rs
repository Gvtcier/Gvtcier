use crate::println;

const IA32_EFER: u32 = 0xC0000080;
const IA32_STAR: u32 = 0xC0000081;
const IA32_LSTAR: u32 = 0xC0000082;
const IA32_FMASK: u32 = 0xC0000084;

#[repr(C)]
struct SyscallFrame {
    r8: u64,
    r11: u64,
    rcx: u64,
    rdx: u64,
    rsi: u64,
    rdi: u64,
    rax: u64,
    r10: u64,
    r9: u64,
}

pub fn init() {
    unsafe {
        let efer = rdmsr(IA32_EFER);
        wrmsr(IA32_EFER, efer | 1);
        wrmsr(IA32_STAR, (0x08u64 << 48) | (0x28u64 << 32));
        wrmsr(IA32_LSTAR, syscall_entry_addr() as u64);
        wrmsr(IA32_FMASK, 0x200);
    }
    println!("syscall ready");
}

core::arch::global_asm!(
    ".global asm_syscall_entry",
    "asm_syscall_entry:",
    "mov rsp, [rip + CUR_RSP0]",
    "push rbp",
    "push rbx",
    "push r12",
    "push r13",
    "push r14",
    "push r15",
    "push r9",
    "push r10",
    "push rax",
    "push rdi",
    "push rsi",
    "push rdx",
    "push rcx",
    "push r11",
    "push r8",
    "mov rdi, rsp",
    "mov rsi, r9",
    "call rust_syscall_handler",
    "pop r8",
    "pop r11",
    "pop rcx",
    "pop rdx",
    "pop rsi",
    "pop rdi",
    "pop rax",
    "pop r10",
    "pop r9",
    "pop r15",
    "pop r14",
    "pop r13",
    "pop r12",
    "pop rbx",
    "pop rbp",
    "push 0x1b",
    "push r8",
    "push r11",
    "push 0x23",
    "push rcx",
    "iretq",
);

extern "C" {
    static asm_syscall_entry: u8;
}

fn syscall_entry_addr() -> usize {
    unsafe { &asm_syscall_entry as *const u8 as usize }
}

const MAX_SYSCALL: usize = 40;
static mut AUDIT: [u64; MAX_SYSCALL + 1] = [0; MAX_SYSCALL + 1];

pub fn syscall_count(nr: u32) -> u64 {
    unsafe {
        if (nr as usize) <= MAX_SYSCALL {
            AUDIT[nr as usize]
        } else {
            0
        }
    }
}

pub fn syscall_total() -> u64 {
    unsafe {
        let mut t = 0u64;
        for i in 1..=MAX_SYSCALL {
            t = t.wrapping_add(AUDIT[i]);
        }
        t
    }
}

#[no_mangle]
extern "C" fn rust_syscall_handler(frame: *const SyscallFrame, out: *mut u64) {
    let f = unsafe { &*frame };
    let nr = f.rax as usize;
    if nr >= 1 && nr <= MAX_SYSCALL {
        unsafe {
            AUDIT[nr] = AUDIT[nr].wrapping_add(1);
        }
    }
    let mut ret = 0u64;
    match f.rax {
        1 => {
            let buf = f.rdi as *const u8;
            let len = f.rsi as usize;
            let s = unsafe { core::slice::from_raw_parts(buf, len) };
            crate::io::Serial::write_bytes(s);
        }
        2 => {
            crate::Task::schedule();
        }
        3 => {
            let ep = crate::Ipc::create(crate::Task::current_id());
            if ep != 0xFFFFFFFF {
                let caps = unsafe { &mut *crate::Task::current_caps() };
                ret = crate::Cap::alloc(
                    caps,
                    crate::Cap::OBJ_ENDPOINT,
                    ep,
                    crate::Cap::RIGHT_SEND | crate::Cap::RIGHT_RECV,
                ) as u64;
            }
        }
        4 => {
            let c = f.rdi as u32;
            let ptr = f.rsi as usize;
            let len = f.rdx as usize;
            let caps = unsafe { &*crate::Task::current_caps() };
            if let Some(cap) = crate::Cap::lookup(caps, c) {
                if cap.obj_type == crate::Cap::OBJ_ENDPOINT
                    && cap.rights & crate::Cap::RIGHT_SEND != 0
                {
                    let mut msg = crate::Ipc::Message {
                        len: len as u32,
                        data: [0; crate::Ipc::MSG_SIZE],
                    };
                    let n = len.min(crate::Ipc::MSG_SIZE);
                    unsafe {
                        core::ptr::copy_nonoverlapping(ptr as *const u8, msg.data.as_mut_ptr(), n);
                    }
                    let r = crate::Ipc::send(cap.obj_id, &msg);
                    if r == 0 {
                        crate::Task::wake_on(cap.obj_id);
                    }
                    ret = r as u64;
                } else {
                    ret = 2;
                }
            } else {
                ret = 2;
            }
        }
        5 => {
            let c = f.rdi as u32;
            let ptr = f.rsi as usize;
            let nonblock = f.rdx != 0;
            let caps = unsafe { &*crate::Task::current_caps() };
            if let Some(cap) = crate::Cap::lookup(caps, c) {
                if cap.obj_type == crate::Cap::OBJ_ENDPOINT
                    && cap.rights & crate::Cap::RIGHT_RECV != 0
                {
                    let mut msg = crate::Ipc::Message {
                        len: 0,
                        data: [0; crate::Ipc::MSG_SIZE],
                    };
                    if crate::Ipc::recv(cap.obj_id, &mut msg) == 0 {
                        unsafe {
                            core::ptr::copy_nonoverlapping(
                                msg.data.as_ptr(),
                                ptr as *mut u8,
                                msg.len as usize,
                            );
                        }
                        ret = msg.len as u64;
                    } else if nonblock {
                        ret = 0;
                    } else {
                        crate::Task::block_on(cap.obj_id);
                        crate::Task::schedule();
                        if crate::Ipc::recv(cap.obj_id, &mut msg) == 0 {
                            unsafe {
                                for i in 0..msg.len as usize {
                                    *(ptr as *mut u8).add(i) = msg.data[i];
                                }
                            }
                            ret = msg.len as u64;
                        }
                    }
                } else {
                    ret = 2;
                }
            } else {
                ret = 2;
            }
        }
        6 => {
            ret = 0x1234;
        }
        7 => {
            let x = f.rdi as usize;
            let y = f.rsi as usize;
            let ch = f.rdx as u8;
            crate::io::Fb::draw_char(x, y, ch);
        }
        8 => {
            let c = f.rdi as u32;
            crate::io::Fb::clear(c);
        }
        9 => {
            let lba = f.rdi as u32;
            let count = f.rsi as u32;
            let ptr = f.rdx as *mut u8;
            let n = (count as usize) * 512;
            let buf = unsafe { core::slice::from_raw_parts_mut(ptr, n) };
            ret = crate::io::Ata::read(lba, count, buf) as u64;
        }
        10 => {
            let caps = f.rdi as u32;
            let ep = f.rsi as u32;
            let task = crate::Task::current_id();
            ret = crate::Drv::register(caps, ep, task) as u64;
        }
        11 => {
            let caps = f.rdi as u32;
            ret = crate::Drv::lookup(caps) as u64;
        }
        12 => {
            let ptr = f.rdi as *const u8;
            let len = f.rsi as u32;
            ret = crate::io::Audio::play(ptr, len) as u64;
        }
        13 => {
            let w = f.rdi as usize;
            let h = f.rsi as usize;
            let buf = f.rdx as usize;
            crate::println!("sc13 w={} h={} buf={:#x}", w, h, buf);
            ret = crate::Gfx::canvas_create(w, h, buf) as u64;
            crate::println!("sc13 ret={}", ret);
        }
        14 => {
            let id = f.rdi as u32;
            crate::Gfx::canvas_destroy(id);
        }
        15 => {
            let id = f.rdi as u32;
            ret = crate::Gfx::canvas_buf(id) as u64;
        }
        16 => {
            let id = f.rdi as u32;
            let x = f.rsi as usize;
            let y = f.rdx as usize;
            crate::Gfx::compose(id, x, y);
        }
        17 => {
            let id = f.rdi as u32;
            crate::Gfx::canvas_destroy(id);
        }
        18 => {}
        22 => {
            let vaddr = f.rdi as usize;
            let phys = f.rsi as usize;
            let pages = f.rdx as usize;
            let flags = f.rcx as u64;
            crate::Vmm::map(crate::mem::Paging::cr3(), vaddr, phys, pages, flags);
        }
        23 => {
            let vaddr = f.rdi as usize;
            let pages = f.rsi as usize;
            crate::Vmm::unmap(crate::mem::Paging::cr3(), vaddr, pages);
        }
        24 => {
            let ticks = f.rdi as u64;
            let start = crate::intr::Apic::tick();
            while crate::intr::Apic::tick().wrapping_sub(start) < ticks {
                crate::Task::schedule();
            }
        }
        25 => {
            let id = f.rdi as u32;
            let w = f.rsi as usize;
            let h = f.rdx as usize;
            let color = f.r10 as u32;
            let buf = crate::Gfx::canvas_buf(id) as *mut u8;
            crate::Gvtcier2D::g2d_fill(buf, w, h, color);
        }
        26 => {
            let id = f.rdi as u32;
            let x = f.rsi as usize;
            let y = f.rdx as usize;
            let rw = f.r10 as usize;
            let rh = f.r9 as usize;
            let color = unsafe { *((f.r8 as usize).wrapping_sub(8) as *const u32) };
            let w = crate::Gfx::canvas_w(id);
            let buf = crate::Gfx::canvas_buf(id) as *mut u8;
            crate::Gvtcier2D::g2d_rect(buf, w, x, y, rw, rh, color);
        }
        27 => {
            let id = f.rdi as u32;
            let x1 = f.rsi as usize;
            let y1 = f.rdx as usize;
            let x2 = f.r10 as usize;
            let y2 = f.r9 as usize;
            let color = unsafe { *((f.r8 as usize).wrapping_sub(8) as *const u32) };
            let w = crate::Gfx::canvas_w(id);
            let buf = crate::Gfx::canvas_buf(id) as *mut u8;
            crate::Gvtcier2D::g2d_line(buf, w, x1, y1, x2, y2, color);
        }
        28 => {
            let id = f.rdi as u32;
            let x = f.rsi as usize;
            let y = f.rdx as usize;
            let ch = f.r10 as u8;
            let color = f.r9 as u32;
            let w = crate::Gfx::canvas_w(id);
            let buf = crate::Gfx::canvas_buf(id) as *mut u8;
            crate::Gvtcier2D::g2d_char(buf, w, x, y, ch, color);
        }
        29 => {
            let id = f.rdi as u32;
            let x = f.rsi as usize;
            let y = f.rdx as usize;
            let color = f.r10 as u32;
            let ptr = f.r9 as usize;
            let mut len = 0usize;
            while unsafe { *(ptr as *const u8).add(len) } != 0 && len < 256 {
                len += 1;
            }
            let text = unsafe { core::slice::from_raw_parts(ptr as *const u8, len) };
            let w = crate::Gfx::canvas_w(id);
            let buf = crate::Gfx::canvas_buf(id) as *mut u8;
            crate::Gvtcier2D::g2d_text_utf8(buf, w, x, y, text, color);
        }
        30 => {
            let name = f.rdi as *const u8;
            let len = f.rsi as usize;
            if name.is_null() || len == 0 || len > 8 {
                ret = 0xFFFFFFFF;
            } else {
                let name_slice = unsafe { core::slice::from_raw_parts(name, len) };
                ret = crate::io::File::open(name_slice) as u64;
            }
        }
        31 => {
            let handle = f.rdi as u32;
            let buf = f.rsi as usize;
            let max = f.rdx as usize;
            if buf == 0 || max > 32768 {
                ret = 0;
            } else {
                ret = crate::io::File::read(handle, buf as *mut u8, max) as u64;
            }
        }
        32 => {
            let handle = f.rdi as u32;
            crate::io::File::close(handle);
        }
        33 => {
            let order = f.rdi as usize;
            if order > crate::mem::Buddy::MAX_ORDER {
                ret = 0xFFFFFFFFFFFFFFFF;
            } else if let Some(p) = crate::mem::alloc_pages(order) {
                ret = p as u64;
            } else {
                ret = 0xFFFFFFFFFFFFFFFF;
            }
        }
        34 => {
            let index = f.rdi as usize;
            let order = f.rsi as usize;
            if order <= crate::mem::Buddy::MAX_ORDER {
                crate::mem::free_pages(index, order);
            }
        }
        35 => {
            let id = f.rdi as u32;
            let pts = f.rsi as usize;
            let color = f.rdx as u32;
            let seg = f.r10 as usize;
            if pts == 0 || seg > 64 {
                ret = 1;
            } else {
                let slice = unsafe { core::slice::from_raw_parts(pts as *const (usize, usize), 12) };
                let w = crate::Gfx::canvas_w(id);
                let buf = crate::Gfx::canvas_buf(id) as *mut u8;
                crate::Gvtcier2D::bezier(buf, w, slice, color, seg);
            }
        }
        36 => {
            let lba = f.rdi as u64;
            let count = f.rsi as u32;
            let ptr = f.rdx as *const u8;
            if ptr.is_null() || count > 32 {
                ret = 1;
            } else {
                let n = (count as usize) * 512;
                let buf = unsafe { core::slice::from_raw_parts(ptr, n) };
                ret = crate::io::Ahci::write(lba, count, buf) as u64;
            }
        }
        37 => {
            let handle = f.rdi as u32;
            let buf = f.rsi as usize;
            let len = f.rdx as usize;
            if buf == 0 || len > 32768 {
                ret = 0;
            } else {
                ret = crate::io::File::write(handle, buf as *const u8, len) as u64;
            }
        }
        38 => {
            let elf_ptr = f.rdi as usize;
            let elf_len = f.rsi as usize;
            let caps_ptr = f.rdx as usize;
            let caps_len = f.rcx as usize;
            if elf_ptr == 0 || elf_len == 0 || elf_len > 0x100000 || caps_ptr == 0 {
                ret = 0xFFFFFFFF;
            } else {
                let elf = unsafe { core::slice::from_raw_parts(elf_ptr as *const u8, elf_len) };
                let parent = crate::Task::current_id();
                let id = crate::Task::create_task(elf, 0, parent);
                if id != 0xFFFFFFFF {
                    let n = core::cmp::min(caps_len, crate::Cap::MAX_CAPS);
                    for i in 0..n {
                        let cap = unsafe { *((caps_ptr + i * 8) as *const crate::Cap::Cap) };
                        crate::Task::set_cap(id, i, cap);
                    }
                }
                ret = id as u64;
            }
        }
        39 => {
            ret = crate::intr::Apic::tick();
        }
        40 => {
            let caps_ptr = f.rdi as *const crate::Cap::Cap;
            let caps_len = f.rsi as usize;
            let mut caps = [crate::Cap::CAP_NONE; crate::Cap::MAX_CAPS];
            let n = core::cmp::min(caps_len, crate::Cap::MAX_CAPS);
            for i in 0..n {
                caps[i] = unsafe { *caps_ptr.add(i) };
            }
            ret = crate::Process::fission(caps) as u64;
        }
        _ => {}
    }
    unsafe {
        if f.rax <= 24 || f.rax == 39 || f.rax == 40 {
            *out = ret;
        }
    }
}

unsafe fn rdmsr(msr: u32) -> u64 {
    let lo: u32;
    let hi: u32;
    core::arch::asm!(
        "rdmsr",
        in("ecx") msr,
        out("eax") lo,
        out("edx") hi,
        options(nomem, nostack),
    );
    ((hi as u64) << 32) | lo as u64
}

unsafe fn wrmsr(msr: u32, v: u64) {
    let lo = v as u32;
    let hi = (v >> 32) as u32;
    core::arch::asm!(
        "wrmsr",
        in("ecx") msr,
        in("eax") lo,
        in("edx") hi,
        options(nomem, nostack),
    );
}
