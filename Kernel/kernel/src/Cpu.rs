const APIC_BASE_MSR: u32 = 0x1B;
const AP_TRAMPOLINE_ADDR: u64 = 0x8000;
const MAX_APS: usize = 8;
static mut AP_STACKS: [[u8; 16384]; MAX_APS] = [[0; 16384]; MAX_APS];
static mut AP_COUNT: usize = 0;
static mut AP_ALIVE: [bool; MAX_APS] = [false; MAX_APS];

pub fn cpu_count() -> usize {
    unsafe { AP_COUNT + 1 }
}

pub fn ap_alive_count() -> usize {
    unsafe {
        let mut n = 0usize;
        for i in 0..MAX_APS {
            if AP_ALIVE[i] {
                n += 1;
            }
        }
        n
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

pub fn apic_base() -> u64 {
    unsafe { rdmsr(APIC_BASE_MSR) & 0xFFFFF000 }
}

pub fn local_apic_id() -> u32 {
    let base = apic_base();
    unsafe {
        let reg = (base + 0x20) as *const u32;
        core::ptr::read_volatile(reg) >> 24
    }
}

extern "C" {
    static ap_trampoline: u8;
    static ap_trampoline_end: u8;
    static ap_cr3: u64;
    static ap_stack_top: u64;
}

#[no_mangle]
extern "C" fn ap_entry() -> ! {
    let id = local_apic_id();
    crate::println!("AP alive id={}", id);
    unsafe {
        for i in 0..MAX_APS {
            if !AP_ALIVE[i] {
                AP_ALIVE[i] = true;
                break;
            }
        }
        crate::intr::Apic::init_timer();
    }
    loop {
        crate::intr::Apic::wait_tick();
        crate::Task::schedule();
    }
}

fn apic_write(reg: u64, v: u32) {
    let base = apic_base();
    unsafe {
        core::ptr::write_volatile((base + reg) as *mut u32, v);
    }
}

fn apic_read(reg: u64) -> u32 {
    let base = apic_base();
    unsafe { core::ptr::read_volatile((base + reg) as *const u32) }
}

fn send_ipi(icr_low: u32) {
    apic_write(0x310, 0xC0000000);
    apic_write(0x300, icr_low);
    while apic_read(0x300) & 0x1000 != 0 {}
}

fn setup_ap_gdt() {
    unsafe {
        let base = 0x9000usize as *mut u64;
        base.write_volatile(0);
        base.add(1).write_volatile(0x00CF9A000000FFFF);
        base.add(2).write_volatile(0x00CF92000000FFFF);
        base.add(3).write_volatile(0x00AF9A000000FFFF);
        let gdtr = 0x8FF8usize as *mut u8;
        gdtr.write_volatile(31);
        gdtr.add(1).write_volatile(0);
        gdtr.add(2).write_volatile(0x00);
        gdtr.add(3).write_volatile(0x90);
        gdtr.add(4).write_volatile(0x00);
        gdtr.add(5).write_volatile(0x00);
    }
}

pub fn start_aps() {
    setup_ap_gdt();
    unsafe {
        let len = (&ap_trampoline_end as *const u8 as usize)
            - (&ap_trampoline as *const u8 as usize);
        core::ptr::copy_nonoverlapping(
            &ap_trampoline as *const u8,
            AP_TRAMPOLINE_ADDR as *mut u8,
            len,
        );
        let base = AP_TRAMPOLINE_ADDR as *mut u8;
        let t = &ap_trampoline as *const u8 as usize;
        let cr3_off = (&ap_cr3 as *const u64 as usize) - t;
        let stack_off = (&ap_stack_top as *const u64 as usize) - t;
        let cr3 = crate::mem::Paging::cr3();
        let stack = AP_STACKS[AP_COUNT].as_ptr() as usize + 16384;
        core::ptr::write_volatile(base.add(cr3_off) as *mut u64, cr3 as u64);
        core::ptr::write_volatile(base.add(stack_off) as *mut u64, stack as u64);
        AP_COUNT += 1;
        send_ipi(0x4500);
        for _ in 0..10000000 {
            core::hint::spin_loop();
        }
        send_ipi(0x4608);
        for _ in 0..1000000 {
            core::hint::spin_loop();
        }
        send_ipi(0x4608);
    }
}
