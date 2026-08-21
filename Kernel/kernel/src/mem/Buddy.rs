use gvtcier_abi::KERNEL_VIRT;

const PAGE_SIZE: usize = 4096;
pub const MAX_ORDER: usize = 12;

pub struct BuddyAllocator {
    base: usize,
    total_pages: usize,
    max_order: usize,
    free_lists: [Option<usize>; MAX_ORDER + 1],
}

impl BuddyAllocator {
    pub const fn new() -> Self {
        BuddyAllocator {
            base: 0,
            total_pages: 0,
            max_order: 0,
            free_lists: [None; MAX_ORDER + 1],
        }
    }

    pub fn init(&mut self, base: usize, total_pages: usize) {
        self.base = base;
        self.total_pages = total_pages;
        let mut mo = 0usize;
        let mut t = total_pages;
        while t > 1 && mo < MAX_ORDER {
            t >>= 1;
            mo += 1;
        }
        self.max_order = mo;
        let mut pages = total_pages;
        let mut order = self.max_order;
        let mut block = 1usize << order;
        let mut index = 0usize;
        while pages > 0 {
            while block > pages {
                order -= 1;
                block = 1usize << order;
            }
            self.push_free(index, order);
            index += block;
            pages -= block;
        }
    }

    pub fn base(&self) -> usize {
        self.base
    }

    pub fn alloc(&mut self, order: usize) -> Option<usize> {
        let mut j = order;
        while j <= self.max_order && self.free_lists[j].is_none() {
            j += 1;
        }
        if j > self.max_order {
            return None;
        }
        let index = self.pop_free(j)?;
        while j > order {
            j -= 1;
            self.push_free(index + (1 << j), j);
        }
        Some(index)
    }

    pub fn free(&mut self, index: usize, order: usize) {
        let mut idx = index;
        let mut ord = order;
        while ord < self.max_order {
            let buddy = idx ^ (1 << ord);
            if !self.remove_free(buddy, ord) {
                break;
            }
            idx = idx.min(buddy);
            ord += 1;
        }
        self.push_free(idx, ord);
    }

    pub fn shell_stats(&self) {
        use crate::io::Serial;
        Serial::print_str("total pages: ");
        Serial::print_hex(self.total_pages as u64);
        Serial::print_str("\r\nfree: ");
        let mut free = 0usize;
        for order in 0..=self.max_order {
            let mut cur = self.free_lists[order];
            while let Some(c) = cur {
                free += 1 << order;
                cur = unsafe {
                    let next = self.free_ptr(c).read();
                    if next == u64::MAX {
                        None
                    } else {
                        Some(next as usize)
                    }
                };
            }
        }
        Serial::print_hex(free as u64);
        Serial::print_str(" pages\r\n");
    }

    fn push_free(&mut self, index: usize, order: usize) {
        let head = self.free_lists[order];
        unsafe {
            self.free_ptr(index).write(head.map(|h| h as u64).unwrap_or(u64::MAX));
        }
        self.free_lists[order] = Some(index);
    }

    fn pop_free(&mut self, order: usize) -> Option<usize> {
        let head = self.free_lists[order]?;
        let next = unsafe { self.free_ptr(head).read() };
        self.free_lists[order] = if next == u64::MAX {
            None
        } else {
            Some(next as usize)
        };
        Some(head)
    }

    fn remove_free(&mut self, index: usize, order: usize) -> bool {
        let mut prev: Option<usize> = None;
        let mut cur = self.free_lists[order];
        while let Some(c) = cur {
            if c == index {
                let next = unsafe { self.free_ptr(c).read() };
                let next = if next == u64::MAX {
                    None
                } else {
                    Some(next as usize)
                };
                match prev {
                    None => self.free_lists[order] = next,
                    Some(p) => unsafe {
                        self.free_ptr(p).write(next.map(|n| n as u64).unwrap_or(u64::MAX));
                    },
                }
                return true;
            }
            let n = unsafe { self.free_ptr(c).read() };
            if n == u64::MAX {
                break;
            }
            prev = Some(c);
            cur = Some(n as usize);
        }
        false
    }

    unsafe fn free_ptr(&self, index: usize) -> *mut u64 {
        let phys = self.base + index * PAGE_SIZE;
        (KERNEL_VIRT as usize + phys) as *mut u64
    }
}
