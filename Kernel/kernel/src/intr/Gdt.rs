use crate::println;

#[repr(C, packed)]
struct GdtPtr {
    limit: u16,
    base: u64,
}

#[repr(C, packed)]
struct TaskStateSegment {
    reserved0: u32,
    rsp0: u64,
    rsp1: u64,
    rsp2: u64,
    reserved1: u64,
    ist: [u64; 7],
    reserved2: u64,
    reserved3: u16,
    iomap_base: u16,
}

pub const KERNEL_CS: u16 = 0x08;
pub const KERNEL_DS: u16 = 0x10;
pub const USER_CS: u16 = 0x20;
pub const USER_DS: u16 = 0x18;
pub const TSS_SEL: u16 = 0x28;

static mut GDT: [u8; 0x38] = [0; 0x38];
static mut TSS: TaskStateSegment = TaskStateSegment {
    reserved0: 0,
    rsp0: 0,
    rsp1: 0,
    rsp2: 0,
    reserved1: 0,
    ist: [0; 7],
    reserved2: 0,
    reserved3: 0,
    iomap_base: 0xFFFF,
};
static mut KERNEL_STACK: [u8; 16384] = [0; 16384];

pub fn init() {
    unsafe {
        set_flat(0x08, 0x9A, 0xAF);
        set_flat(0x10, 0x92, 0xCF);
        set_flat(0x18, 0xF2, 0xCF);
        set_flat(0x20, 0xFA, 0xAF);
        TSS.rsp0 = ((&KERNEL_STACK as *const u8 as usize) + 16384) as u64;
        let tss_addr = &TSS as *const TaskStateSegment as u64;
        set_tss_desc(0x28, tss_addr, 0x67);
        let ptr = GdtPtr {
            limit: 0x38 - 1,
            base: &GDT as *const u8 as u64,
        };
        core::arch::asm!("lgdt [{}]", in(reg) &ptr, options(readonly, nostack, preserves_flags));
        core::arch::asm!(
            "push {sel}",
            "lea {tmp}, [rip + 2f]",
            "push {tmp}",
            "retfq",
            "2:",
            sel = in(reg) KERNEL_CS as u64,
            tmp = out(reg) _,
            options(nostack),
        );
        core::arch::asm!(
            "mov ds, {0}",
            "mov es, {0}",
            "mov ss, {0}",
            in(reg) KERNEL_DS as u64,
            options(nostack),
        );
        core::arch::asm!("ltr ax", in("ax") TSS_SEL, options(nostack));
    }
}

pub fn set_tss_rsp0(v: u64) {
    unsafe {
        TSS.rsp0 = v;
    }
}

pub fn dump() {
    unsafe {
        let mut gdtr = GdtPtr { limit: 0, base: 0 };
        core::arch::asm!("sgdt [{0}]", in(reg) &mut gdtr, options(nostack));
        let limit = core::ptr::addr_of!(gdtr.limit).read_unaligned();
        let base = core::ptr::addr_of!(gdtr.base).read_unaligned();
        println!("gdt: limit={:#x} base={:#x}", limit, base);
        let g = base as *const u8;
        for off in [0x08usize, 0x18usize, 0x28usize] {
            let mut b = [0u8; 16];
            for i in 0..16 {
                b[i] = g.add(off + i).read();
            }
            println!(
                "gdt @{:#x}: {:02x} {:02x} {:02x} {:02x} {:02x} {:02x} {:02x} {:02x} | {:02x} {:02x} {:02x} {:02x} {:02x} {:02x} {:02x} {:02x}",
                off,
                b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7],
                b[8], b[9], b[10], b[11], b[12], b[13], b[14], b[15]
            );
        }
        let rsp0 = core::ptr::addr_of!(TSS.rsp0).read_unaligned();
        println!("tss: rsp0={:#x}", rsp0);
    }
}

unsafe fn set_flat(offset: usize, access: u8, flags: u8) {
    let g = &mut GDT;
    g[offset] = 0xFF;
    g[offset + 1] = 0xFF;
    g[offset + 2] = 0;
    g[offset + 3] = 0;
    g[offset + 4] = 0;
    g[offset + 5] = access;
    g[offset + 6] = flags;
    g[offset + 7] = 0;
}

unsafe fn set_tss_desc(offset: usize, base: u64, limit: u16) {
    let g = &mut GDT;
    g[offset] = limit as u8;
    g[offset + 1] = (limit >> 8) as u8;
    g[offset + 2] = base as u8;
    g[offset + 3] = (base >> 8) as u8;
    g[offset + 4] = (base >> 16) as u8;
    g[offset + 5] = 0x89;
    g[offset + 6] = 0x00;
    g[offset + 7] = (base >> 24) as u8;
    g[offset + 8] = (base >> 32) as u8;
    g[offset + 9] = (base >> 40) as u8;
    g[offset + 10] = (base >> 48) as u8;
    g[offset + 11] = (base >> 56) as u8;
    for i in 0..4 {
        g[offset + 12 + i] = 0;
    }
}
