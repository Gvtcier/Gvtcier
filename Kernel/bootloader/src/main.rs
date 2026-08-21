#![no_std]
#![no_main]

extern crate alloc;

use alloc::vec::Vec;
use gvtcier_abi::{BootInfo, MemoryRegion};
use uefi::boot::{AllocateType, SearchType};
use uefi::cstr16;
use uefi::fs::Path;
use uefi::helpers;
use uefi::mem::memory_map::{MemoryMap, MemoryType};
use uefi::prelude::*;
use uefi::proto::console::gop::{GraphicsOutput, PixelFormat};
use uefi::proto::media::fs::SimpleFileSystem;

const PAGE_SIZE: u64 = 0x1000;
const TWO_MB: u64 = 0x200000;

#[entry]
fn efi_main() -> Status {
    helpers::init().expect("uefi init failed");
    uefi::println!("Gvtcier bootloader: UEFI started");

    let gop_handle = match uefi::boot::locate_handle_buffer(
        SearchType::from_proto::<GraphicsOutput>(),
    ) {
        Ok(handles) if !handles.is_empty() => handles[0],
        _ => {
            uefi::println!("no GOP found");
            return Status::ABORTED;
        }
    };

    let mut gop = match uefi::boot::open_protocol_exclusive::<GraphicsOutput>(gop_handle) {
        Ok(g) => g,
        Err(_) => {
            uefi::println!("open GOP failed");
            return Status::ABORTED;
        }
    };

    let modes: Vec<_> = gop.modes().collect();
    for mode in &modes {
        let (w, h) = mode.info().resolution();
        if w == 1920 && h == 1080 {
            let _ = gop.set_mode(mode);
            break;
        }
    }

    let (fb_w, fb_h) = gop.current_mode_info().resolution();
    let fb_stride_px = gop.current_mode_info().stride();
    let fb_format = match gop.current_mode_info().pixel_format() {
        PixelFormat::Rgb => 0u32,
        PixelFormat::Bgr => 1u32,
        _ => 2u32,
    };
    let fb_addr = gop.frame_buffer().as_mut_ptr() as u64;
    uefi::println!(
        "GOP: {}x{} stride={} addr={:#x}",
        fb_w,
        fb_h,
        fb_stride_px,
        fb_addr
    );

    let sfs_handle = match uefi::boot::locate_handle_buffer(
        SearchType::from_proto::<SimpleFileSystem>(),
    ) {
        Ok(handles) if !handles.is_empty() => handles[0],
        _ => {
            uefi::println!("no file system found");
            return Status::ABORTED;
        }
    };
    let sfs = match uefi::boot::open_protocol_exclusive::<SimpleFileSystem>(sfs_handle) {
        Ok(s) => s,
        Err(_) => {
            uefi::println!("open file system failed");
            return Status::ABORTED;
        }
    };
    let mut fs = uefi::fs::FileSystem::new(sfs);
    let kernel_bytes = match fs.read(Path::new(cstr16!("\\KERNEL.GCX"))) {
        Ok(b) => b,
        Err(_) => {
            let mut found = None;
            if let Ok(handles) = uefi::boot::locate_handle_buffer(
                SearchType::from_proto::<SimpleFileSystem>(),
            ) {
                for h in handles.iter().skip(1) {
                    if let Ok(s) = uefi::boot::open_protocol_exclusive::<SimpleFileSystem>(*h) {
                        let mut f2 = uefi::fs::FileSystem::new(s);
                        if let Ok(b) = f2.read(Path::new(cstr16!("\\KERNEL.GCX"))) {
                            found = Some(b);
                            break;
                        }
                    }
                }
            }
            match found {
                Some(b) => b,
                None => {
                    uefi::println!("read KERNEL.GCX failed: not found on any fs");
                    return Status::ABORTED;
                }
            }
        }
    };
    uefi::println!("kernel size: {}", kernel_bytes.len());

    let entry = match load_gcx(&kernel_bytes) {
        core::result::Result::Ok(e) => e,
        core::result::Result::Err(e) => {
            uefi::println!("GCX load failed: {}", e);
            return Status::ABORTED;
        }
    };
    uefi::println!("kernel loaded, entry = {:#x}", entry);

    let bootinfo_mem = uefi::boot::allocate_pages(
        AllocateType::AnyPages,
        MemoryType::LOADER_DATA,
        1,
    )
    .expect("alloc bootinfo");
    let bootinfo_addr = bootinfo_mem.as_ptr() as u64;
    let regions_mem = uefi::boot::allocate_pages(
        AllocateType::AnyPages,
        MemoryType::LOADER_DATA,
        4,
    )
    .expect("alloc regions");
    let regions_addr = regions_mem.as_ptr() as u64;
    let pt_mem = uefi::boot::allocate_pages(
        AllocateType::AnyPages,
        MemoryType::LOADER_DATA,
        7,
    )
    .expect("alloc page tables");
    let page_tables = pt_mem.as_ptr() as u64;

    let pml4 = page_tables;
    let pdpt_lo = page_tables + PAGE_SIZE;
    let pdpt_hi = page_tables + 2 * PAGE_SIZE;
    let pd_base = page_tables + 3 * PAGE_SIZE;
    unsafe {
        let p4 = pml4 as *mut u64;
        for i in 0..512 {
            p4.add(i).write_volatile(0);
        }
        p4.write_volatile(pdpt_lo | 0x3);
        p4.add(256).write_volatile(pdpt_hi | 0x3);
        let p3lo = pdpt_lo as *mut u64;
        let p3hi = pdpt_hi as *mut u64;
        for i in 0..512 {
            p3lo.add(i).write_volatile(0);
            p3hi.add(i).write_volatile(0);
        }
        for g in 0..4usize {
            let g64 = g as u64;
            p3lo.add(g).write_volatile((pd_base + g64 * PAGE_SIZE) | 0x3);
            p3hi.add(g).write_volatile((pd_base + g64 * PAGE_SIZE) | 0x3);
            let pd = (pd_base + g64 * PAGE_SIZE) as *mut u64;
            for i in 0..512usize {
                let addr = (g64 * 512 + i as u64) * TWO_MB;
                pd.add(i).write_volatile(addr | 0x83);
            }
        }
    }

    let mem_map = unsafe { uefi::boot::exit_boot_services(Some(MemoryType::LOADER_DATA)) };

    let map_len = mem_map.len();
    let mut n = 0u64;
    for i in 0..map_len {
        let d = &mem_map[i];
        let kind = match d.ty {
            MemoryType::CONVENTIONAL => MemoryRegion::KIND_USABLE,
            _ => MemoryRegion::KIND_RESERVED,
        };
        let r = MemoryRegion {
            start: d.phys_start,
            len: d.page_count * 4096,
            kind,
        };
        unsafe {
            let p = (regions_addr + n * core::mem::size_of::<MemoryRegion>() as u64)
                as *mut MemoryRegion;
            p.write(r);
        }
        n += 1;
    }
    core::mem::forget(mem_map);

    let bootinfo = BootInfo {
        mem_map_addr: regions_addr,
        mem_map_len: n,
        fb_addr,
        fb_width: fb_w as u32,
        fb_height: fb_h as u32,
        fb_stride: (fb_stride_px * 4) as u32,
        fb_pixel_format: fb_format,
    };
    unsafe {
        (bootinfo_addr as *mut BootInfo).write(bootinfo);
    }

    unsafe {
        core::arch::asm!(
            "cli",
            "mov cr3, {0}",
            "mov rdi, {1}",
            "jmp {2}",
            in(reg) pml4,
            in(reg) bootinfo_addr,
            in(reg) entry,
            options(noreturn),
        );
    }
}

fn load_gcx(data: &[u8]) -> core::result::Result<u64, &'static str> {
    if data.len() < 64 || u64::from_le_bytes(data[0..8].try_into().unwrap()) != 0x786367 {
        return Err("bad gcx magic");
    }
    let seg_num = u32::from_le_bytes(data[12..16].try_into().unwrap()) as usize;
    let entry = u64::from_le_bytes(data[16..24].try_into().unwrap());
    let virt_base = u64::from_le_bytes(data[24..32].try_into().unwrap());
    let seg_off = u64::from_le_bytes(data[32..40].try_into().unwrap()) as usize;
    for i in 0..seg_num {
        let off = seg_off + i * 40;
        if off + 40 > data.len() {
            return Err("truncated segment table");
        }
        let p_offset = u64::from_le_bytes(data[off..off + 8].try_into().unwrap());
        let p_vaddr = u64::from_le_bytes(data[off + 8..off + 16].try_into().unwrap());
        let p_filesz = u64::from_le_bytes(data[off + 16..off + 24].try_into().unwrap());
        let p_memsz = u64::from_le_bytes(data[off + 24..off + 32].try_into().unwrap());
        let src = data
            .get(p_offset as usize..(p_offset + p_filesz) as usize)
            .ok_or("bad file range")?;
        let dst = p_vaddr - virt_base;
        unsafe {
            core::ptr::copy_nonoverlapping(src.as_ptr(), dst as *mut u8, p_filesz as usize);
            if p_memsz > p_filesz {
                core::ptr::write_bytes(
                    (dst + p_filesz) as *mut u8,
                    0,
                    (p_memsz - p_filesz) as usize,
                );
            }
        }
    }
    Ok(entry)
}
