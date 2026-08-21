#![no_std]
#![no_main]

extern crate alloc;

use alloc::vec::Vec;
use core::arch::global_asm;
use core::panic::PanicInfo;

use gvtcier_abi::{BootInfo, KERNEL_VIRT};

mod Cap;
mod Cpu;
mod Device;
mod Drv;
mod Gfx;
mod Spinlock;
mod Gvtcier2D;
mod Time;
mod Vmm;
mod intr;
mod io;
mod Ipc;
mod mem;
mod Process;
mod Task;

const PERSON_ELF_KBD: &[u8] =
    include_bytes!("../../../out/x86_64-unknown-none/release/gvtcier-kbd-drv");
const PERSON_ELF_MOUSE: &[u8] =
    include_bytes!("../../../out/x86_64-unknown-none/release/gvtcier-mouse-drv");
const PERSON_ELF_DRAW: &[u8] =
    include_bytes!("../../../out/x86_64-unknown-none/release/gvtcier-draw-demo");
static mut FS_BUF: [u8; 16384] = [0; 16384];

global_asm!(
    ".global _start",
    "_start:",
    "lea rsp, [rip + _stack_top]",
    "call gvtcier_entry",
    ".Lhlt:",
    "hlt",
    "jmp .Lhlt",
);

#[no_mangle]
extern "C" fn gvtcier_entry(info: *const BootInfo) -> ! {
    io::Serial::init();
    println!("Gvtcier kernel alive");
    let boot = unsafe { &*info };
    mem::init(boot);
    mem::selftest();
    mem::Heap::HEAP.init();
    let mut v: Vec<u64> = Vec::new();
    for i in 0..100 {
        v.push(i);
    }
    let s: u64 = v.iter().sum();
    println!("heap vec len={} sum={}", v.len(), s);
    mem::Paging::selftest();
    intr::Gdt::init();
    intr::Gdt::dump();
    intr::Idt::init();
    intr::Apic::init();
    intr::Syscall::init();
    println!("intr ready");
    let mut m = Ipc::Message {
        len: 5,
        data: [0; Ipc::MSG_SIZE],
    };
    m.data[0..5].copy_from_slice(b"hello");
    let ep = Ipc::create(1);
    let s = Ipc::send(ep, &m);
    let mut out = Ipc::Message {
        len: 0,
        data: [0; Ipc::MSG_SIZE],
    };
    let r = Ipc::recv(ep, &mut out);
    println!(
        "ipc: ep={} send={} recv={} msg={}",
        ep,
        s,
        r,
        core::str::from_utf8(&out.data[..out.len as usize]).unwrap()
    );
    let kep = Ipc::create(2);
    println!("kbd ep: {}", kep);
    let ar = io::Audio::init();
    println!("audio: init={} nabm={:#x}", ar, io::Audio::nabm());
    let mut hyp = [0u8; 24 + 882 * 2];
    hyp[0..3].copy_from_slice(b"HYP");
    hyp[8..12].copy_from_slice(&44100u32.to_le_bytes());
    hyp[12..16].copy_from_slice(&16u32.to_le_bytes());
    hyp[16..20].copy_from_slice(&1u32.to_le_bytes());
    hyp[20..24].copy_from_slice(&(882u32 * 2).to_le_bytes());
    for i in 0..882usize {
        let v: i16 = if (i / 50) % 2 == 0 { 8000 } else { -8000 };
        let b = v.to_le_bytes();
        hyp[24 + i * 2] = b[0];
        hyp[24 + i * 2 + 1] = b[1];
    }
    let pr = io::Audio::play_hyp(&hyp);
    println!("audio play: ret={}", pr);
    if boot.fb_addr != 0 && boot.fb_width > 0 && boot.fb_height > 0 {
        io::Fb::init(boot.fb_addr, boot.fb_width, boot.fb_height, boot.fb_stride);
        io::Fb::clear(0xFFC89000);
    }
    for a in (0..0x100000).step_by(4096) {
        crate::mem::Paging::map_page(crate::mem::Paging::cr3(), a, a, 0x3);
    }
    Cpu::start_aps();
    crate::io::Ahci::init();
    crate::io::Gvinter::init();
    crate::io::Gvinter::net_test();
    let ping_start = crate::intr::Apic::tick();
    while crate::intr::Apic::tick().wrapping_sub(ping_start) < 2000 {
        crate::io::Gvinter::poll();
        core::hint::spin_loop();
    }
    Task::init_main();
    crate::Process::fission([crate::Cap::CAP_NONE; crate::Cap::MAX_CAPS]);
    Task::create_task(PERSON_ELF_MOUSE, 0, 0);
    Task::create_task(PERSON_ELF_DRAW, 0, 0);
    {
        let names: [&[u8]; 3] = [b"A.TXT", b"B.TXT", b"C.TXT"];
        for name in names {
            let h = crate::io::File::open(name);
            if h != 0xFFFFFFFF {
                let mut buf = [0u8; 64];
                let r = crate::io::File::read(h, buf.as_mut_ptr(), buf.len());
                println!(
                    "fs: {}: {} bytes: {}",
                    core::str::from_utf8(name).unwrap_or("?"),
                    r,
                    core::str::from_utf8(&buf[..r as usize]).unwrap_or("?")
                );
                crate::io::File::close(h);
            } else {
                println!("fs: {}: open fail", core::str::from_utf8(name).unwrap_or("?"));
            }
        }
    }
    unsafe {
        let h = crate::io::File::open(b"BIG.TXT");
        if h != 0xFFFFFFFF {
            let r = crate::io::File::read(h, FS_BUF.as_mut_ptr(), FS_BUF.len());
            let mut ok = r == 10240;
            if ok {
                for i in 0..r as usize {
                    if FS_BUF[i] != (i % 251) as u8 {
                        ok = false;
                        break;
                    }
                }
            }
            println!("fs: BIG.TXT: {} bytes verify={}", r, if ok { "ok" } else { "FAIL" });
            crate::io::File::close(h);
        } else {
            println!("fs: BIG.TXT: open fail");
        }
    }
    {
        let name = b"TEST.TXT";
        let h = crate::io::File::open(name);
        if h != 0xFFFFFFFF {
            let src = b"Gvtcier write test v0.2";
            let mut gv = [0u8; 128];
            let gn = crate::io::Gv2280::utf8_to_gv(src, &mut gv);
            let n = crate::io::File::write(h, gv.as_ptr(), gn);
            println!("write test: wrote {} bytes", n);
            let mut buf = [0u8; 64];
            let r = crate::io::File::read(h, buf.as_mut_ptr(), 64);
            let mut utf = [0u8; 128];
            let un = crate::io::Gv2280::gv_to_utf8(&buf[..r as usize], &mut utf);
            println!(
                "write test: read back {}: {}",
                un,
                core::str::from_utf8(&utf[..un as usize]).unwrap_or("?")
            );
            crate::io::File::close(h);
        } else {
            println!("write test: open fail");
        }
    }
    crate::io::Shell::selftest();
    crate::io::Shell::run();
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("Gvtcier kernel panic: {}", info);
    let mut d = [0u64; 16];
    unsafe {
        core::arch::asm!(
            "mov {0}, rax",
            "mov {1}, rbx",
            "mov {2}, rcx",
            "mov {3}, rdx",
            "mov {4}, rsi",
            "mov {5}, rdi",
            "mov {6}, rbp",
            "mov {7}, r8",
            "mov {8}, r9",
            "mov {9}, r10",
            "mov {10}, r11",
            "mov {11}, r12",
            "mov {12}, r13",
            "mov {13}, r14",
            "mov {14}, r15",
            out(reg) d[0],
            out(reg) d[1],
            out(reg) d[2],
            out(reg) d[3],
            out(reg) d[4],
            out(reg) d[5],
            out(reg) d[6],
            out(reg) d[7],
            out(reg) d[8],
            out(reg) d[9],
            out(reg) d[10],
            out(reg) d[11],
            out(reg) d[12],
            out(reg) d[13],
            out(reg) d[14],
            options(nostack)
        );
        core::arch::asm!(
            "mov {0}, cr2",
            out(reg) d[15],
            options(nostack)
        );
    }
    println!(
        "dump: rax={:#x} rbx={:#x} rcx={:#x} rdx={:#x}",
        d[0], d[1], d[2], d[3]
    );
    println!(
        "dump: rsi={:#x} rdi={:#x} rbp={:#x} r8={:#x}",
        d[4], d[5], d[6], d[7]
    );
    println!(
        "dump: r9={:#x} r10={:#x} r11={:#x} r12={:#x}",
        d[8], d[9], d[10], d[11]
    );
    println!(
        "dump: r13={:#x} r14={:#x} r15={:#x} cr2={:#x}",
        d[12], d[13], d[14], d[15]
    );
    loop {
        hlt();
    }
}

fn hlt() {
    unsafe { core::arch::asm!("hlt", options(nomem, nostack)) }
}
