use gvtcier_abi::{BootInfo, KERNEL_VIRT, MemoryRegion};

pub mod Buddy;
pub mod Heap;
pub mod Paging;

use crate::println;

static mut BUDDY: Buddy::BuddyAllocator = Buddy::BuddyAllocator::new();

static mut TOTAL_PAGES: usize = 0;
static mut USED_PAGES: usize = 0;

pub fn init(boot: &BootInfo) {
    let regions = unsafe {
        core::slice::from_raw_parts(
            (boot.mem_map_addr + KERNEL_VIRT) as *const MemoryRegion,
            boot.mem_map_len as usize,
        )
    };
    let mut best_start = 0usize;
    let mut best_pages = 0usize;
    for r in regions {
        if r.kind == MemoryRegion::KIND_USABLE && r.start >= 0x1000000 {
            let pages = (r.len / 4096) as usize;
            if pages > best_pages {
                best_pages = pages;
                best_start = r.start as usize;
            }
        }
    }
    println!("mem: buddy region {:#x} pages={}", best_start, best_pages);
    unsafe {
        BUDDY.init(best_start, best_pages);
        TOTAL_PAGES = best_pages;
    }
}

pub fn alloc_pages(order: usize) -> Option<usize> {
    let r = unsafe { BUDDY.alloc(order) };
    if let Some(_) = r {
        unsafe { USED_PAGES += 1 << order; }
    }
    r
}

pub fn region_base() -> usize {
    unsafe { BUDDY.base() }
}

pub fn free_pages(index: usize, order: usize) {
    unsafe { BUDDY.free(index, order) }
    unsafe { USED_PAGES = USED_PAGES.saturating_sub(1 << order); }
}

pub fn total_pages() -> usize {
    unsafe { TOTAL_PAGES }
}

pub fn used_pages() -> usize {
    unsafe { USED_PAGES }
}

pub fn free_pages_total() -> usize {
    unsafe { TOTAL_PAGES.saturating_sub(USED_PAGES) }
}

pub fn selftest() {
    let a = alloc_pages(0).expect("buddy oom");
    let b = alloc_pages(2).expect("buddy oom");
    let c = alloc_pages(0).expect("buddy oom");
    println!(
        "buddy selftest: a={:#x} b={:#x} c={:#x}",
        a, b, c
    );
    free_pages(c, 0);
    free_pages(b, 2);
    free_pages(a, 0);
    let d = alloc_pages(3).expect("buddy merge failed");
    println!("buddy selftest ok: order3={:#x}", d);
    free_pages(d, 3);
    println!(
        "mem stats: total={} used={} free={}",
        total_pages(),
        used_pages(),
        free_pages_total()
    );
}

static mut KASLR_OFFSET: u64 = 0;

fn kaslr_seed() -> u64 {
    let (_, _, sec) = crate::Time::now();
    let tick = crate::intr::Apic::tick();
    let tsc: u64;
    unsafe {
        core::arch::asm!("rdtsc", out("eax") tsc as u32, out("edx") tsc as u32, options(nomem, nostack));
    }
    let _ = tsc;
    (sec as u64).wrapping_mul(0x9E3779B1) ^ tick.wrapping_mul(0x85EBCA6B) ^ 0x2A1171
}

pub fn kaslr_init() {
    unsafe {
        KASLR_OFFSET = (kaslr_seed() % 512) * 4096;
    }
    println!("kaslr: offset={:#x}", unsafe { KASLR_OFFSET });
}

pub fn kaslr_offset() -> u64 {
    unsafe { KASLR_OFFSET }
}

pub fn kaslr_selftest() {
    let page = alloc_pages(0).expect("kaslr oom");
    let phys = region_base() + page * 4096;
    let vaddr = (KERNEL_VIRT as u64 + kaslr_offset() + 0x100000) as usize;
    Paging::map_page(Paging::cr3(), vaddr, phys, Paging::FLAG_WRITABLE);
    unsafe {
        let p = vaddr as *mut u64;
        p.write_volatile(0xC0FFEE);
        let v = p.read_volatile();
        if v == 0xC0FFEE {
            println!("kaslr selftest ok: vaddr={:#x}", vaddr);
        } else {
            println!("kaslr selftest FAIL: {:#x}", v);
        }
    }
    Paging::unmap_page(Paging::cr3(), vaddr);
    free_pages(page, 0);
}
