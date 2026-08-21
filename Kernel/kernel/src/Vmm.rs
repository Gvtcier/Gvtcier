use crate::mem::Paging;

pub fn map(cr3: usize, vaddr: usize, phys: usize, pages: usize, flags: u64) {
    if vaddr & 0xFFF != 0 || phys & 0xFFF != 0 {
        return;
    }
    for i in 0..pages {
        Paging::map_page(cr3, vaddr + i * 4096, phys + i * 4096, flags);
    }
}

pub fn unmap(cr3: usize, vaddr: usize, pages: usize) {
    if vaddr & 0xFFF != 0 {
        return;
    }
    for i in 0..pages {
        Paging::unmap_page(cr3, vaddr + i * 4096);
        unsafe {
            core::arch::asm!("invlpg [rax]", in("rax") vaddr + i * 4096, options(nomem, nostack));
        }
    }
}

pub fn query(cr3: usize, vaddr: usize) -> Option<usize> {
    Paging::page_phys(cr3, vaddr)
}

pub fn map_aligned(cr3: usize, vaddr: usize, phys: usize, pages: usize, flags: u64) -> bool {
    if vaddr & 0xFFF != 0 || phys & 0xFFF != 0 {
        return false;
    }
    map(cr3, vaddr, phys, pages, flags);
    true
}

pub fn unmap_aligned(cr3: usize, vaddr: usize, pages: usize) -> bool {
    if vaddr & 0xFFF != 0 {
        return false;
    }
    unmap(cr3, vaddr, pages);
    true
}
