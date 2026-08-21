use core::alloc::{GlobalAlloc, Layout};
use core::cell::UnsafeCell;
use core::ptr;

use gvtcier_abi::KERNEL_VIRT;

use crate::mem;
use crate::println;

#[repr(C)]
struct FreeBlock {
    size: usize,
    next: *mut FreeBlock,
}

struct HeapState {
    head: *mut FreeBlock,
    alloc_count: u64,
    fail_count: u64,
    free_bytes: usize,
}

pub struct HeapAllocator {
    state: UnsafeCell<HeapState>,
}

impl HeapAllocator {
    pub const fn new() -> Self {
        HeapAllocator {
            state: UnsafeCell::new(HeapState {
                head: ptr::null_mut(),
                alloc_count: 0,
                fail_count: 0,
                free_bytes: 0,
            }),
        }
    }

    pub fn init(&self) {
        let total = mem::total_pages();
        let mut heap_pages = total / 4;
        if heap_pages > 16384 {
            heap_pages = 16384;
        }
        if heap_pages < 1024 {
            heap_pages = 1024;
        }
        let mut order = 0usize;
        let mut p = heap_pages;
        while p > 1 {
            p >>= 1;
            order += 1;
        }
        let heap_size = (1usize << order) * 4096;
        let index = mem::alloc_pages(order).expect("heap alloc failed");
        let phys = mem::region_base() + index * 4096;
        let virt = KERNEL_VIRT as usize + phys;
        unsafe {
            let block = virt as *mut FreeBlock;
            (*block).size = heap_size;
            (*block).next = ptr::null_mut();
            (*self.state.get()).head = block;
            (*self.state.get()).free_bytes = heap_size;
        }
        println!("heap: {:#x} size={}", virt, heap_size);
    }

    pub fn stats(&self) -> (u64, u64, usize) {
        let st = unsafe { &*self.state.get() };
        (st.alloc_count, st.fail_count, st.free_bytes)
    }
}

unsafe impl GlobalAlloc for HeapAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if layout.align() > 16 {
            return ptr::null_mut();
        }
        let st = &mut *self.state.get();
        let need = align16(layout.size().max(1)) + 16;
        let mut prev: *mut FreeBlock = ptr::null_mut();
        let mut cur = st.head;
        while !cur.is_null() {
            if (*cur).size >= need + 16 {
                let rest = (cur as *mut u8).add(need) as *mut FreeBlock;
                (*rest).size = (*cur).size - need;
                (*rest).next = (*cur).next;
                (*cur).size = need;
                (*cur).next = ptr::null_mut();
                if prev.is_null() {
                    st.head = rest;
                } else {
                    (*prev).next = rest;
                }
                st.alloc_count += 1;
                st.free_bytes -= need;
                return (cur as *mut u8).add(16);
            }
            if (*cur).size >= need {
                if prev.is_null() {
                    st.head = (*cur).next;
                } else {
                    (*prev).next = (*cur).next;
                }
                st.alloc_count += 1;
                st.free_bytes -= need;
                return (cur as *mut u8).add(16);
            }
            prev = cur;
            cur = (*cur).next;
        }
        st.fail_count += 1;
        ptr::null_mut()
    }

    unsafe fn dealloc(&self, ptr: *mut u8, _layout: Layout) {
        let st = &mut *self.state.get();
        let block = (ptr as *mut FreeBlock).sub(1);
        (*block).next = st.head;
        st.head = block;
        st.free_bytes += (*block).size;
        let mut changed = true;
        while changed {
            changed = false;
            let mut cur = st.head;
            while !cur.is_null() {
                let next = (*cur).next;
                if !next.is_null() && (cur as usize) + (*cur).size == next as usize {
                    (*cur).size += (*next).size;
                    (*cur).next = (*next).next;
                    changed = true;
                } else {
                    cur = next;
                }
            }
        }
    }
}

fn align16(v: usize) -> usize {
    (v + 15) & !15
}

unsafe impl Sync for HeapAllocator {}

#[global_allocator]
pub static HEAP: HeapAllocator = HeapAllocator::new();
