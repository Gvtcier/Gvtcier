use crate::intr::Gdt::KERNEL_CS;

#[repr(C)]
#[derive(Clone, Copy)]
struct IdtEntry {
    offset_lo: u16,
    selector: u16,
    ist: u8,
    attr: u8,
    offset_mid: u16,
    offset_hi: u32,
    zero: u32,
}

#[repr(C, packed)]
struct IdtPtr {
    limit: u16,
    base: u64,
}

const GATE_INTERRUPT: u8 = 0x8E;

static mut IDT: [IdtEntry; 256] = [IdtEntry {
    offset_lo: 0,
    selector: 0,
    ist: 0,
    attr: 0,
    offset_mid: 0,
    offset_hi: 0,
    zero: 0,
}; 256];

pub fn init() {
    unsafe {
        for v in 0..32 {
            let offset = crate::intr::Exn::addr(v);
            set(&mut IDT[v], offset, KERNEL_CS, GATE_INTERRUPT);
        }
        let ptr = IdtPtr {
            limit: (core::mem::size_of::<IdtEntry>() * 256 - 1) as u16,
            base: &IDT as *const IdtEntry as u64,
        };
        core::arch::asm!("lidt [{}]", in(reg) &ptr, options(readonly, nostack, preserves_flags));
    }
}

pub fn set_vector(vector: usize, offset: usize, selector: u16, attr: u8) {
    unsafe {
        set(&mut IDT[vector], offset, selector, attr);
    }
}

unsafe fn set(entry: &mut IdtEntry, offset: usize, selector: u16, attr: u8) {
    entry.offset_lo = offset as u16;
    entry.offset_mid = (offset >> 16) as u16;
    entry.offset_hi = (offset >> 32) as u32;
    entry.selector = selector;
    entry.ist = 0;
    entry.attr = attr;
    entry.zero = 0;
}
