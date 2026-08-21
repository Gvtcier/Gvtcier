use core::sync::atomic::{AtomicU64, Ordering};

use gvtcier_abi::KERNEL_VIRT;

use crate::intr::Exn::InterruptFrame;
use crate::intr::Gdt::KERNEL_CS;
use crate::intr::Idt;
use crate::println;

const LAPIC_BASE: u64 = 0xFEE00000;
const APIC_BASE_MSR: u32 = 0x1B;
const TIMER_VECTOR: u32 = 32;

static TICK: AtomicU64 = AtomicU64::new(0);
const TIMESLICE: u64 = 100;
static mut LAST_TSC: u64 = 0;
static mut MOUSE_BUF: [u8; 3] = [0; 3];
static mut MOUSE_IDX: usize = 0;

static mut AP_TIMER_ENABLED: bool = false;

pub fn init_timer() {
    unsafe {
        if AP_TIMER_ENABLED {
            return;
        }
        AP_TIMER_ENABLED = true;
        lapic_write(0x320, 0x20000 | TIMER_VECTOR);
        lapic_write(0x3E0, 0x0B);
        lapic_write(0x380, 0x100000);
        Idt::set_vector(TIMER_VECTOR as usize, timer_irq_addr(), KERNEL_CS, 0x8E);
        LAST_TSC = rdtsc();
    }
    unsafe {
        core::arch::asm!("sti", options(nomem, nostack));
    }
}

pub fn wait_tick() {
    loop {
        core::hint::spin_loop();
    }
}

pub fn tick() -> u64 {
    TICK.load(Ordering::Relaxed)
}

pub fn init() {
    unsafe {
        let mut apic_base = rdmsr(APIC_BASE_MSR);
        if apic_base & (1 << 10) != 0 {
            apic_base &= !(1 << 10);
            apic_base &= !(1 << 11);
            wrmsr(APIC_BASE_MSR, apic_base);
            apic_base |= 1 << 11;
            wrmsr(APIC_BASE_MSR, apic_base);
        } else if apic_base & (1 << 11) == 0 {
            apic_base |= 1 << 11;
            wrmsr(APIC_BASE_MSR, apic_base);
        }
        lapic_write(0xF0, 0x1FF);
        outb(0x21, 0xFF);
        outb(0xA1, 0xFF);
        lapic_write(0x320, 0x20000 | TIMER_VECTOR);
        lapic_write(0x3E0, 0x0B);
        lapic_write(0x380, 0x100000);
        Idt::set_vector(TIMER_VECTOR as usize, timer_irq_addr(), KERNEL_CS, 0x8E);
        crate::intr::Ioapic::init();
        Idt::set_vector(33, kbd_irq_addr(), KERNEL_CS, 0x8E);
        Idt::set_vector(44, mouse_irq_addr(), KERNEL_CS, 0x8E);
        init_mouse();
        LAST_TSC = rdtsc();
    }
    println!("apic timer ready");
    unsafe {
        core::arch::asm!("sti", options(nomem, nostack));
    }
}

core::arch::global_asm!(
    ".global asm_timer_irq",
    "asm_timer_irq:",
    "push 0",
    "push 32",
    "jmp irq_common",
    ".global asm_kbd_irq",
    "asm_kbd_irq:",
    "push 0",
    "push 33",
    "jmp irq_common",
    ".global asm_mouse_irq",
    "asm_mouse_irq:",
    "push 0",
    "push 44",
    "jmp irq_common",
    "irq_common:",
    "push rax",
    "push rcx",
    "push rdx",
    "push rbx",
    "push rbp",
    "push rsi",
    "push rdi",
    "push r8",
    "push r9",
    "push r10",
    "push r11",
    "push r12",
    "push r13",
    "push r14",
    "push r15",
    "mov rdi, rsp",
    "call rust_irq_handler",
    "pop r15",
    "pop r14",
    "pop r13",
    "pop r12",
    "pop r11",
    "pop r10",
    "pop r9",
    "pop r8",
    "pop rdi",
    "pop rsi",
    "pop rbp",
    "pop rbx",
    "pop rdx",
    "pop rcx",
    "pop rax",
    "add rsp, 16",
    "iretq",
);

extern "C" {
    static asm_timer_irq: u8;
    static asm_kbd_irq: u8;
    static asm_mouse_irq: u8;
}

fn timer_irq_addr() -> usize {
    unsafe { &asm_timer_irq as *const u8 as usize }
}

fn kbd_irq_addr() -> usize {
    unsafe { &asm_kbd_irq as *const u8 as usize }
}

fn mouse_irq_addr() -> usize {
    unsafe { &asm_mouse_irq as *const u8 as usize }
}

#[no_mangle]
extern "C" fn rust_irq_handler(frame: *const InterruptFrame) {
    let f = unsafe { &*frame };
    if f.vector == 32 {
        unsafe {
            lapic_write(0xB0, 0);
            let tsc = rdtsc();
            let delta = tsc - LAST_TSC;
            LAST_TSC = tsc;
            TICK.fetch_add(1, Ordering::Relaxed);
            if TICK.load(Ordering::Relaxed) % TIMESLICE == 0 {
                crate::Task::schedule();
            }
            crate::Task::timer_poll();
            crate::Task::signal_poll();
            if TICK.load(Ordering::Relaxed) % 50 == 0 {
                println!(
                    "tick={} tsc_delta={}",
                    TICK.load(Ordering::Relaxed),
                    delta
                );
            }
        }
    } else if f.vector == 33 {
        unsafe {
            let sc = inb(0x60);
            crate::io::Keyboard::push(sc);
            let mut msg = crate::Ipc::Message {
                len: 2,
                data: [0; crate::Ipc::MSG_SIZE],
            };
            msg.data[0] = 1;
            msg.data[1] = sc;
            let r = crate::Ipc::send(1, &msg);
            if r == 0 {
                crate::Task::wake_on(1);
            }
            lapic_write(0xB0, 0);
            println!("kbd sc={:#x}", sc);
        }
    } else if f.vector == 44 {
        unsafe {
            let b = inb(0x60);
            if MOUSE_IDX == 0 && b & 0x08 == 0 {
                lapic_write(0xB0, 0);
                return;
            }
            MOUSE_BUF[MOUSE_IDX] = b;
            MOUSE_IDX += 1;
            if MOUSE_IDX >= 3 {
                MOUSE_IDX = 0;
                let mut msg = crate::Ipc::Message {
                    len: 4,
                    data: [0; crate::Ipc::MSG_SIZE],
                };
                msg.data[0] = 2;
                msg.data[1] = MOUSE_BUF[0];
                msg.data[2] = MOUSE_BUF[1];
                msg.data[3] = MOUSE_BUF[2];
                let r = crate::Ipc::send(3, &msg);
                if r == 0 {
                    crate::Task::wake_on(3);
                }
                println!(
                    "mouse dx={} dy={} b={:#x}",
                    MOUSE_BUF[1] as i8,
                    MOUSE_BUF[2] as i8,
                    MOUSE_BUF[0]
                );
            }
            lapic_write(0xB0, 0);
        }
    }
}

fn init_mouse() {
    unsafe {
        outb(0x64, 0xA8);
        wait_in();
        outb(0x64, 0x20);
        wait_out();
        let mut cmd = inb(0x60);
        cmd |= 0x02;
        wait_in();
        outb(0x64, 0x60);
        wait_in();
        outb(0x60, cmd);
        wait_in();
        outb(0x64, 0xD4);
        wait_in();
        outb(0x60, 0xF4);
        wait_out();
        let _ack = inb(0x60);
    }
}

fn wait_in() {
    unsafe {
        for _ in 0..1000 {
            if inb(0x64) & 0x02 == 0 {
                return;
            }
        }
    }
}

fn wait_out() {
    unsafe {
        for _ in 0..1000 {
            if inb(0x64) & 0x01 != 0 {
                return;
            }
        }
    }
}

unsafe fn lapic_write(reg: u64, v: u32) {
    let p = (KERNEL_VIRT + LAPIC_BASE + reg) as *mut u32;
    p.write_volatile(v);
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

fn rdtsc() -> u64 {
    let lo: u32;
    let hi: u32;
    unsafe {
        core::arch::asm!("rdtsc", out("eax") lo, out("edx") hi, options(nomem, nostack));
    }
    ((hi as u64) << 32) | lo as u64
}

unsafe fn outb(port: u16, v: u8) {
    core::arch::asm!("out dx, al", in("dx") port, in("al") v, options(nomem, nostack));
}

unsafe fn inb(port: u16) -> u8 {
    let v: u8;
    core::arch::asm!("in al, dx", in("dx") port, out("al") v, options(nomem, nostack));
    v
}
