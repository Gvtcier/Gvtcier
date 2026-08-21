use gvtcier_abi::KERNEL_VIRT;

use crate::Cap::{Cap, CAP_NONE, MAX_CAPS};
use crate::intr::Gdt;
use crate::intr::Gdt::{USER_CS, USER_DS};
use crate::mem;
use crate::mem::Paging;
use crate::println;

pub const MAX_TASKS: usize = 6;
const TASK_KSTACK_SIZE: usize = 16384;
const USER_STACK_BASE: usize = 0x7FFC000;
const USER_STACK_TOP: usize = 0x8000000;

#[repr(C)]
#[derive(Clone, Copy, PartialEq)]
pub enum TaskState {
    Dead = 0,
    Ready = 1,
    Blocked = 2,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct TaskContext {
    pub rax: u64,
    pub rcx: u64,
    pub rdx: u64,
    pub rbx: u64,
    pub rbp: u64,
    pub rsi: u64,
    pub rdi: u64,
    pub r8: u64,
    pub r9: u64,
    pub r10: u64,
    pub r11: u64,
    pub r12: u64,
    pub r13: u64,
    pub r14: u64,
    pub r15: u64,
    pub rflags: u64,
    pub rsp: u64,
    pub rip: u64,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Task {
    pub id: u32,
    pub pid: u32,
    pub parent: u32,
    pub state: TaskState,
    pub priority: u32,
    pub pml4: usize,
    pub kstack_top: u64,
    pub entry: u64,
    pub user_rsp: u64,
    pub caps: [Cap; MAX_CAPS],
    pub blocked_on: i32,
    pub pending_sig: u32,
    pub context: TaskContext,
}

impl Task {
    pub const fn dead() -> Self {
        Task {
            id: 0,
            pid: 0,
            parent: 0,
            state: TaskState::Dead,
            priority: 0,
            pml4: 0,
            kstack_top: 0,
            entry: 0,
            user_rsp: 0,
            caps: [CAP_NONE; MAX_CAPS],
            blocked_on: -1,
            pending_sig: 0,
            context: TaskContext {
                rax: 0,
                rcx: 0,
                rdx: 0,
                rbx: 0,
                rbp: 0,
                rsi: 0,
                rdi: 0,
                r8: 0,
                r9: 0,
                r10: 0,
                r11: 0,
                r12: 0,
                r13: 0,
                r14: 0,
                r15: 0,
                rflags: 0,
                rsp: 0,
                rip: 0,
            },
        }
    }
}

static mut TASKS: [Task; MAX_TASKS] = [Task::dead(); MAX_TASKS];
static mut CURRENT: usize = 0;
static mut NEXT_ID: u32 = 1;
static mut SW_CUR: usize = 0;
static mut TASK_STACKS: [[u8; TASK_KSTACK_SIZE]; MAX_TASKS] = [[0; TASK_KSTACK_SIZE]; MAX_TASKS];
static mut COW_ZERO_PHYS: usize = 0;

const MAX_TIMERS: usize = 8;
static mut TIMERS: [(u32, u64, bool); MAX_TIMERS] = [(0, 0, false); MAX_TIMERS];

pub fn timer_set(task: u32, ticks: u64) -> u32 {
    unsafe {
        let now = crate::intr::Apic::tick();
        for i in 0..MAX_TIMERS {
            if !TIMERS[i].2 {
                TIMERS[i] = (task, now.wrapping_add(ticks), true);
                return i as u32;
            }
        }
    }
    0xFFFFFFFF
}

pub fn timer_cancel(handle: u32) {
    unsafe {
        if (handle as usize) < MAX_TIMERS {
            TIMERS[handle as usize].2 = false;
        }
    }
}

pub fn timer_poll() {
    unsafe {
        let now = crate::intr::Apic::tick();
        for i in 0..MAX_TIMERS {
            if TIMERS[i].2 && now.wrapping_sub(TIMERS[i].1) < 0x80000000 {
                let task = TIMERS[i].0;
                TIMERS[i].2 = false;
                wake_on(task);
            }
        }
    }
}

pub const SIG_TERM: u32 = 9;

pub fn signal_send(task: u32, sig: u32) -> u32 {
    unsafe {
        for i in 0..MAX_TASKS {
            if TASKS[i].id == task && TASKS[i].state != TaskState::Dead {
                TASKS[i].pending_sig = sig;
                return 0;
            }
        }
    }
    1
}

pub fn signal_poll() {
    unsafe {
        for i in 0..MAX_TASKS {
            if TASKS[i].state != TaskState::Dead && TASKS[i].pending_sig != 0 {
                let sig = TASKS[i].pending_sig;
                TASKS[i].pending_sig = 0;
                if sig == SIG_TERM {
                    TASKS[i].state = TaskState::Dead;
                }
            }
        }
    }
}

#[no_mangle]
pub static mut CUR_RSP0: u64 = 0;

pub fn set_cap(id: u32, idx: usize, cap: Cap) {
    unsafe {
        for i in 0..MAX_TASKS {
            if TASKS[i].id == id && TASKS[i].state != TaskState::Dead {
                if idx < MAX_CAPS {
                    TASKS[i].caps[idx] = cap;
                }
                return;
            }
        }
    }
}

pub fn init_main() {
    unsafe {
        let t = &mut TASKS[0];
        t.id = 0;
        t.pid = 0;
        t.parent = 0;
        t.state = TaskState::Ready;
        t.priority = 0;
        t.pml4 = crate::mem::Paging::cr3();
        t.kstack_top = main_stack_top();
        t.blocked_on = -1;
    }
}

extern "C" {
    static _stack_top: u8;
}

fn main_stack_top() -> u64 {
    unsafe { (&_stack_top as *const u8 as usize) as u64 }
}

pub fn create_task(elf: &[u8], pid: u32, parent: u32) -> u32 {
    unsafe {
        let mut slot = None;
        for i in 1..MAX_TASKS {
            if TASKS[i].state == TaskState::Dead {
                slot = Some(i);
                break;
            }
        }
        let slot = slot.expect("no task slot");
        let t = &mut TASKS[slot];
        t.id = NEXT_ID;
        t.pid = pid;
        t.parent = parent;
        NEXT_ID += 1;
        t.priority = 0;

        let pml4_page = mem::alloc_pages(0).expect("oom");
        let pml4_phys = mem::region_base() + pml4_page * 4096;
        let pdpt_page = mem::alloc_pages(0).expect("oom");
        let pdpt_phys = mem::region_base() + pdpt_page * 4096;

        let user_pml4 = (KERNEL_VIRT as usize + pml4_phys) as *mut u64;
        for i in 0..512 {
            user_pml4.add(i).write_volatile(0);
        }
        let kp = (KERNEL_VIRT as usize + Paging::cr3()) as *const u64;
        user_pml4.add(256).write_volatile(kp.add(256).read());
        user_pml4.add(0).write_volatile(pdpt_phys as u64 | 0x7);
        let pdpt = (KERNEL_VIRT as usize + pdpt_phys) as *mut u64;
        for i in 0..512 {
            pdpt.add(i).write_volatile(0);
        }

        let entry = load_elf(elf, pml4_phys);

        unsafe {
            if COW_ZERO_PHYS == 0 {
                let zp = mem::alloc_pages(0).expect("oom");
                COW_ZERO_PHYS = mem::region_base() + zp * 4096;
                let z = COW_ZERO_PHYS as *mut u8;
                for i in 0..4096 {
                    z.add(i).write_volatile(0);
                }
            }
        }
        for page in (USER_STACK_BASE + 4096..USER_STACK_TOP).step_by(4096) {
            Paging::map_page_cow(pml4_phys, page, COW_ZERO_PHYS);
        }

        t.pml4 = pml4_phys;
        t.kstack_top = ((&TASK_STACKS[slot] as *const u8 as usize) + TASK_KSTACK_SIZE) as u64;
        t.entry = entry;
        t.user_rsp = USER_STACK_TOP as u64;
        t.caps[1] = Cap {
            obj_type: crate::Cap::OBJ_ENDPOINT,
            obj_id: 0,
            rights: crate::Cap::RIGHT_SEND | crate::Cap::RIGHT_RECV,
        };
        t.caps[2] = Cap {
            obj_type: crate::Cap::OBJ_ENDPOINT,
            obj_id: 1,
            rights: crate::Cap::RIGHT_RECV,
        };
        t.caps[3] = Cap {
            obj_type: crate::Cap::OBJ_ENDPOINT,
            obj_id: 2,
            rights: crate::Cap::RIGHT_SEND | crate::Cap::RIGHT_RECV,
        };
        t.caps[4] = Cap {
            obj_type: crate::Cap::OBJ_ENDPOINT,
            obj_id: 3,
            rights: crate::Cap::RIGHT_SEND | crate::Cap::RIGHT_RECV,
        };
        t.state = TaskState::Ready;
        t.context.rsp = t.kstack_top;
        t.context.rip = task_entry as usize as u64;

        println!(
            "task {} created: entry={:#x} pml4={:#x} caps={}",
            t.id, entry, pml4_phys, MAX_CAPS
        );
        t.id
    }
}

core::arch::global_asm!(
    ".global asm_switch",
    "asm_switch:",
    "mov [rdi + 0], rax",
    "mov [rdi + 8], rcx",
    "mov [rdi + 16], rdx",
    "mov [rdi + 24], rbx",
    "mov [rdi + 32], rbp",
    "mov [rdi + 40], rsi",
    "mov [rdi + 48], rdi",
    "mov [rdi + 56], r8",
    "mov [rdi + 64], r9",
    "mov [rdi + 72], r10",
    "mov [rdi + 80], r11",
    "mov [rdi + 88], r12",
    "mov [rdi + 96], r13",
    "mov [rdi + 104], r14",
    "mov [rdi + 112], r15",
    "pushfq",
    "pop rax",
    "mov [rdi + 120], rax",
    "lea rax, [rsp + 8]",
    "mov [rdi + 128], rax",
    "mov rax, [rsp]",
    "mov [rdi + 136], rax",
    "mov rsp, [rsi + 128]",
    "push [rsi + 120]",
    "popfq",
    "push qword ptr [rsi + 136]",
    "mov rax, [rsi + 0]",
    "mov rcx, [rsi + 8]",
    "mov rdx, [rsi + 16]",
    "mov rbx, [rsi + 24]",
    "mov rbp, [rsi + 32]",
    "mov r8, [rsi + 56]",
    "mov r9, [rsi + 64]",
    "mov r10, [rsi + 72]",
    "mov r11, [rsi + 80]",
    "mov r12, [rsi + 88]",
    "mov r13, [rsi + 96]",
    "mov r14, [rsi + 104]",
    "mov r15, [rsi + 112]",
    "mov rdi, [rsi + 48]",
    "mov rsi, [rsi + 40]",
    "ret",
);

extern "C" {
    fn asm_switch(prev: *mut TaskContext, next: *const TaskContext);
}

pub fn terminate() {
    unsafe {
        TASKS[CURRENT].state = TaskState::Dead;
    }
}

pub fn shell_tasks() {
    unsafe {
        use crate::io::Serial;
        for i in 1..MAX_TASKS {
            let t = &TASKS[i];
            if t.state != TaskState::Dead {
                let st = match t.state {
                    TaskState::Ready => "Ready",
                    TaskState::Blocked => "Blocked",
                    _ => "Dead",
                };
                Serial::print_hex(t.id as u64);
                Serial::print_str(" ");
                Serial::print_hex(t.pid as u64);
                Serial::print_str(" ");
                Serial::print_str(st);
                Serial::print_str(" ");
                Serial::print_hex(t.priority as u64);
                Serial::print_str("\r\n");
            }
        }
    }
}

pub fn shell_kill(tid: u32) {
    unsafe {
        use crate::io::Serial;
        for i in 1..MAX_TASKS {
            if TASKS[i].id == tid && TASKS[i].state != TaskState::Dead {
                TASKS[i].state = TaskState::Dead;
                Serial::print_str("killed\r\n");
                return;
            }
        }
        Serial::print_str("not found\r\n");
    }
}

pub fn schedule() {
    unsafe {
        let cur = CURRENT;
        let mut best = usize::MAX;
        let mut best_prio: u32 = 0;
        let mut found = false;
        for i in 1..=MAX_TASKS {
            let next = (cur + i) % MAX_TASKS;
            if TASKS[next].state == TaskState::Ready {
                let p = TASKS[next].priority;
                if !found || p > best_prio {
                    found = true;
                    best = next;
                    best_prio = p;
                }
            }
        }
        if found {
            CURRENT = best;
            SW_CUR = cur;
            CUR_RSP0 = TASKS[best].kstack_top;
            Gdt::set_tss_rsp0(TASKS[best].kstack_top);
            Paging::set_cr3(TASKS[best].pml4);
            asm_switch(&mut TASKS[SW_CUR].context, &TASKS[best].context);
        }
    }
}

pub fn start() -> ! {
    unsafe {
        for i in 0..MAX_TASKS {
            if TASKS[i].state == TaskState::Ready {
                CURRENT = i;
                CUR_RSP0 = TASKS[i].kstack_top;
                Gdt::set_tss_rsp0(TASKS[i].kstack_top);
                Paging::set_cr3(TASKS[i].pml4);
                asm_switch(&mut TASKS[0].context, &TASKS[i].context);
                break;
            }
        }
    }
    loop {
        unsafe { core::arch::asm!("hlt", options(nomem, nostack)) }
    }
}

pub fn current_id() -> u32 {
    unsafe { TASKS[CURRENT].id }
}

pub fn current_caps() -> *mut [Cap; MAX_CAPS] {
    unsafe { &mut TASKS[CURRENT].caps }
}

pub fn block_on(ep: u32) {
    unsafe {
        TASKS[CURRENT].state = TaskState::Blocked;
        TASKS[CURRENT].blocked_on = ep as i32;
    }
}

pub fn wake_on(ep: u32) {
    unsafe {
        for i in 0..MAX_TASKS {
            if TASKS[i].state == TaskState::Blocked && TASKS[i].blocked_on == ep as i32 {
                TASKS[i].state = TaskState::Ready;
                TASKS[i].blocked_on = -1;
            }
        }
    }
}

fn task_entry() -> ! {
    unsafe {
        let t = &TASKS[CURRENT];
        let pml4 = t.pml4;
        let entry = t.entry;
        let user_rsp = t.user_rsp as usize;
        let kstack_top = t.kstack_top;
        CUR_RSP0 = kstack_top;
        Gdt::set_tss_rsp0(kstack_top);
        enter_user(entry, user_rsp, pml4);
    }
}

fn load_elf(data: &[u8], pml4: usize) -> u64 {
    let e_entry = u64::from_le_bytes(data[0x18..0x20].try_into().unwrap());
    let e_phoff = u64::from_le_bytes(data[0x20..0x28].try_into().unwrap()) as usize;
    let e_phentsize = u16::from_le_bytes(data[0x36..0x38].try_into().unwrap()) as usize;
    let e_phnum = u16::from_le_bytes(data[0x38..0x3A].try_into().unwrap()) as usize;
    for i in 0..e_phnum {
        let off = e_phoff + i * e_phentsize;
        if off + 56 > data.len() {
            break;
        }
        let p_type = u32::from_le_bytes(data[off..off + 4].try_into().unwrap());
        if p_type != 1 {
            continue;
        }
        let p_offset = u64::from_le_bytes(data[off + 8..off + 16].try_into().unwrap()) as usize;
        let p_vaddr = u64::from_le_bytes(data[off + 16..off + 24].try_into().unwrap()) as usize;
        let p_filesz = u64::from_le_bytes(data[off + 32..off + 40].try_into().unwrap()) as usize;
        let p_memsz = u64::from_le_bytes(data[off + 40..off + 48].try_into().unwrap()) as usize;
        if p_offset + p_filesz > data.len() {
            continue;
        }
        let start = p_vaddr & !0xFFF;
        let end = (p_vaddr + p_memsz + 0xFFF) & !0xFFF;
        for page in (start..end).step_by(4096) {
            let phys = match Paging::page_phys(pml4, page) {
                Some(p) => p,
                None => {
                    let p = mem::alloc_pages(0).expect("oom");
                    let phys = mem::region_base() + p * 4096;
                    Paging::map_page(
                        pml4,
                        page,
                        phys,
                        Paging::FLAG_USER | Paging::FLAG_WRITABLE,
                    );
                    phys
                }
            };
            let dst = (KERNEL_VIRT as usize + phys) as *mut u8;
            unsafe {
                let copy_lo = page.max(p_vaddr);
                let copy_hi = (page + 4096).min(p_vaddr + p_filesz);
                if copy_lo < copy_hi {
                    let src = data.as_ptr().add(p_offset + (copy_lo - p_vaddr));
                    core::ptr::copy_nonoverlapping(src, dst.add(copy_lo - page), copy_hi - copy_lo);
                }
                let zero_lo = page.max(p_vaddr + p_filesz);
                let zero_hi = (page + 4096).min(p_vaddr + p_memsz);
                if zero_lo < zero_hi {
                    core::ptr::write_bytes(dst.add(zero_lo - page), 0, zero_hi - zero_lo);
                }
            }
        }
    }
    e_entry
}

fn enter_user(entry: u64, user_rsp: usize, pml4: usize) -> ! {
    unsafe {
        core::arch::asm!(
            "mov cr3, {cr3}",
            "push {ss}",
            "push {rsp}",
            "push {rflags}",
            "push {cs}",
            "push {rip}",
            "iretq",
            cr3 = in(reg) pml4 as u64,
            ss = in(reg) USER_DS as u64 | 3,
            rsp = in(reg) user_rsp as u64,
            rflags = in(reg) 0x202u64,
            cs = in(reg) USER_CS as u64 | 3,
            rip = in(reg) entry,
            options(noreturn),
        );
    }
}
