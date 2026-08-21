use gvtcier_abi::KERNEL_VIRT;

use crate::mem;
use crate::println;

pub const FLAG_WRITABLE: u64 = 1 << 1;
pub const FLAG_USER: u64 = 1 << 2;

fn virt(phys: usize) -> usize {
    KERNEL_VIRT as usize + phys
}

pub fn cr3() -> usize {
    let v: u64;
    unsafe { core::arch::asm!("mov {0}, cr3", out(reg) v, options(nomem, nostack)) }
    v as usize
}

pub fn set_cr3(phys: usize) {
    unsafe { core::arch::asm!("mov cr3, {0}", in(reg) phys as u64, options(nomem, nostack)) }
}

pub fn map_page(cr3: usize, vaddr: usize, phys: usize, flags: u64) {
    if vaddr & 0xFFF != 0 || phys & 0xFFF != 0 {
        println!("paging: map unaligned vaddr={:#x} phys={:#x}", vaddr, phys);
        return;
    }
    let pml4_idx = (vaddr >> 39) & 0x1FF;
    let pdpt_idx = (vaddr >> 30) & 0x1FF;
    let pd_idx = (vaddr >> 21) & 0x1FF;
    let pt_idx = (vaddr >> 12) & 0x1FF;
    let pml4 = virt(cr3) as *mut u64;
    unsafe {
        let pdpt = match next_level(pml4, pml4_idx, flags, 1) {
            Some(p) => p,
            None => return,
        };
        let pd = match next_level(pdpt, pdpt_idx, flags, 2) {
            Some(p) => p,
            None => return,
        };
        let pde = pd.add(pd_idx).read_volatile();
        let pt = if pde & 1 != 0 && pde & (1 << 7) != 0 {
            let base = (pde & !0xFFF) as usize;
            let page = match mem::alloc_pages(0) {
                Some(p) => p,
                None => {
                    println!("paging: oom");
                    return;
                }
            };
            let pt_phys = mem::region_base() + page * 4096;
            let p = pt_phys as *mut u64;
            for i in 0..512 {
                p.add(i).write_volatile((base + i * 4096) as u64 | 0x3);
            }
            pd.add(pd_idx).write_volatile(pt_phys as u64 | 0x3);
            p
        } else {
            match next_level(pd, pd_idx, flags, 3) {
                Some(p) => p,
                None => return,
            }
        };
        pt.add(pt_idx)
            .write_volatile((phys as u64 & !0xFFF) | flags | 1);
    }
    invlpg(vaddr);
}

pub fn unmap_page(cr3: usize, vaddr: usize) {
    let pml4_idx = (vaddr >> 39) & 0x1FF;
    let pdpt_idx = (vaddr >> 30) & 0x1FF;
    let pd_idx = (vaddr >> 21) & 0x1FF;
    let pt_idx = (vaddr >> 12) & 0x1FF;
    let pml4 = virt(cr3) as *mut u64;
    unsafe {
        let pdpt = walk(pml4, pml4_idx);
        if pdpt.is_null() {
            return;
        }
        let pd = walk(pdpt, pdpt_idx);
        if pd.is_null() {
            return;
        }
        let pt = walk(pd, pd_idx);
        if pt.is_null() {
            return;
        }
        pt.add(pt_idx).write_volatile(0);
    }
    invlpg(vaddr);
}

pub fn map_page_cow(cr3: usize, vaddr: usize, phys: usize) {
    if vaddr & 0xFFF != 0 || phys & 0xFFF != 0 {
        println!("paging: cow map unaligned vaddr={:#x} phys={:#x}", vaddr, phys);
        return;
    }
    let pml4_idx = (vaddr >> 39) & 0x1FF;
    let pdpt_idx = (vaddr >> 30) & 0x1FF;
    let pd_idx = (vaddr >> 21) & 0x1FF;
    let pt_idx = (vaddr >> 12) & 0x1FF;
    let pml4 = virt(cr3) as *mut u64;
    unsafe {
        let pdpt = match next_level(pml4, pml4_idx, FLAG_USER, 1) {
            Some(p) => p,
            None => return,
        };
        let pd = match next_level(pdpt, pdpt_idx, FLAG_USER, 2) {
            Some(p) => p,
            None => return,
        };
        let pde = pd.add(pd_idx).read_volatile();
        let pt = if pde & 1 != 0 && pde & (1 << 7) != 0 {
            let base = (pde & !0xFFF) as usize;
            let page = match mem::alloc_pages(0) {
                Some(p) => p,
                None => {
                    println!("paging: oom");
                    return;
                }
            };
            let pt_phys = mem::region_base() + page * 4096;
            let p = pt_phys as *mut u64;
            for i in 0..512 {
                p.add(i).write_volatile((base + i * 4096) as u64 | 0x3);
            }
            pd.add(pd_idx).write_volatile(pt_phys as u64 | 0x3);
            p
        } else {
            match next_level(pd, pd_idx, FLAG_USER, 3) {
                Some(p) => p,
                None => return,
            }
        };
        pt.add(pt_idx)
            .write_volatile((phys as u64 & !0xFFF) | FLAG_USER | (1 << 9) | 1);
    }
    invlpg(vaddr);
}

pub fn cow_fault(cr3: usize, vaddr: usize) -> bool {
    let pml4_idx = (vaddr >> 39) & 0x1FF;
    let pdpt_idx = (vaddr >> 30) & 0x1FF;
    let pd_idx = (vaddr >> 21) & 0x1FF;
    let pt_idx = (vaddr >> 12) & 0x1FF;
    let pml4 = virt(cr3) as *mut u64;
    unsafe {
        let pdpt = walk(pml4, pml4_idx);
        if pdpt.is_null() {
            return false;
        }
        let pd = walk(pdpt, pdpt_idx);
        if pd.is_null() {
            return false;
        }
        let pt = walk(pd, pd_idx);
        if pt.is_null() {
            return false;
        }
        let e = pt.add(pt_idx).read_volatile();
        if e & 1 == 0 || e & (1 << 9) == 0 {
            return false;
        }
        let old_phys = (e & !0xFFF) as usize;
        let page = match mem::alloc_pages(0) {
            Some(p) => p,
            None => return false,
        };
        let new_phys = mem::region_base() + page * 4096;
        core::ptr::copy_nonoverlapping(old_phys as *const u8, new_phys as *mut u8, 4096);
        pt.add(pt_idx)
            .write_volatile((new_phys as u64) | FLAG_USER | FLAG_WRITABLE | 1);
    }
    invlpg(vaddr);
    true
}

unsafe fn next_level(table: *mut u64, idx: usize, flags: u64, depth: usize) -> Option<*mut u64> {
    if depth > 3 {
        println!("paging: walk depth exceeded");
        return None;
    }
    let entry = table.add(idx).read_volatile();
    if entry & 1 != 0 {
        return Some(virt((entry & !0xFFF) as usize) as *mut u64);
    }
    let page = match mem::alloc_pages(0) {
        Some(p) => p,
        None => {
            println!("paging: oom");
            return None;
        }
    };
    let phys = mem::region_base() + page * 4096;
    let p = virt(phys) as *mut u64;
    for i in 0..512 {
        p.add(i).write_volatile(0);
    }
    table.add(idx).write_volatile((phys as u64) | flags | 0x3);
    Some(p)
}

unsafe fn walk(table: *mut u64, idx: usize) -> *mut u64 {
    let entry = table.add(idx).read_volatile();
    if entry & 1 != 0 {
        virt((entry & !0xFFF) as usize) as *mut u64
    } else {
        core::ptr::null_mut()
    }
}

pub fn page_phys(cr3: usize, vaddr: usize) -> Option<usize> {
    let pml4_idx = (vaddr >> 39) & 0x1FF;
    let pdpt_idx = (vaddr >> 30) & 0x1FF;
    let pd_idx = (vaddr >> 21) & 0x1FF;
    let pt_idx = (vaddr >> 12) & 0x1FF;
    let pml4 = virt(cr3) as *mut u64;
    unsafe {
        let pdpt = walk(pml4, pml4_idx);
        if pdpt.is_null() {
            return None;
        }
        let pd = walk(pdpt, pdpt_idx);
        if pd.is_null() {
            return None;
        }
        let pt = walk(pd, pd_idx);
        if pt.is_null() {
            return None;
        }
        let e = pt.add(pt_idx).read_volatile();
        if e & 1 != 0 {
            Some((e & !0xFFF) as usize)
        } else {
            None
        }
    }
}

pub fn invlpg(vaddr: usize) {
    unsafe {
        core::arch::asm!(
            "invlpg [{0}]",
            in(reg) vaddr,
            options(nomem, nostack, preserves_flags),
        )
    }
}

pub fn selftest() {
    let cr3 = cr3();
    let page = mem::alloc_pages(0).expect("oom");
    let phys = mem::region_base() + page * 4096;
    let vaddr = KERNEL_VIRT as usize + 0x1_0000_0000usize;
    map_page(cr3, vaddr, phys, FLAG_WRITABLE);
    unsafe {
        let p = vaddr as *mut u64;
        p.write_volatile(0x1234_5678);
        let v = p.read_volatile();
        if v == 0x1234_5678 {
            println!(
                "paging selftest ok: vaddr={:#x} phys={:#x}",
                vaddr, phys
            );
        } else {
            println!("paging selftest FAIL: {:#x}", v);
        }
    }
    unmap_page(cr3, vaddr);
}
