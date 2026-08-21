use gvtcier_abi::KERNEL_VIRT;

use crate::println;

const IOAPIC_BASE: u64 = 0xFEC00000;
const IOREGSEL: u64 = 0x00;
const IOREGWIN: u64 = 0x10;
const KBD_IRQ: u32 = 1;
const KBD_VECTOR: u32 = 33;
const MOUSE_IRQ: u32 = 12;
const MOUSE_VECTOR: u32 = 44;

pub fn init() {
    unsafe {
        let ver = io_read(0x1);
        let max_rte = ((ver >> 16) & 0xFF) + 1;
        println!("ioapic: ver={:#x} max_rte={}", ver, max_rte);
        let lo = KBD_VECTOR;
        let hi = 0u32;
        io_write(0x10 + 2 * KBD_IRQ, lo);
        io_write(0x10 + 2 * KBD_IRQ + 1, hi);
        let rte = io_read(0x10 + 2 * KBD_IRQ);
        println!("ioapic: irq{} rte_lo={:#x}", KBD_IRQ, rte);
        io_write(0x10 + 2 * MOUSE_IRQ, MOUSE_VECTOR);
        io_write(0x10 + 2 * MOUSE_IRQ + 1, hi);
        let rte2 = io_read(0x10 + 2 * MOUSE_IRQ);
        println!("ioapic: irq{} rte_lo={:#x}", MOUSE_IRQ, rte2);
    }
}

unsafe fn io_read(index: u32) -> u32 {
    let sel = (KERNEL_VIRT + IOAPIC_BASE + IOREGSEL) as *mut u32;
    let win = (KERNEL_VIRT + IOAPIC_BASE + IOREGWIN) as *mut u32;
    sel.write_volatile(index);
    win.read_volatile()
}

unsafe fn io_write(index: u32, v: u32) {
    let sel = (KERNEL_VIRT + IOAPIC_BASE + IOREGSEL) as *mut u32;
    let win = (KERNEL_VIRT + IOAPIC_BASE + IOREGWIN) as *mut u32;
    sel.write_volatile(index);
    win.write_volatile(v);
}
