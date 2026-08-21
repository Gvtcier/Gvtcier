#![no_std]

pub mod Sys;

pub const KERNEL_VIRT: u64 = 0xFFFF800000000000;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct BootInfo {
    pub mem_map_addr: u64,
    pub mem_map_len: u64,
    pub fb_addr: u64,
    pub fb_width: u32,
    pub fb_height: u32,
    pub fb_stride: u32,
    pub fb_pixel_format: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct MemoryRegion {
    pub start: u64,
    pub len: u64,
    pub kind: u32,
}

impl MemoryRegion {
    pub const KIND_USABLE: u32 = 0;
    pub const KIND_RESERVED: u32 = 1;
}
