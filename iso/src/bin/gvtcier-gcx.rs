use std::env;
use std::fs;

const KERNEL_VIRT: u64 = 0xFFFF800000000000;
const GCX_MAGIC: u64 = 0x786367;
const GCX_VERSION: u32 = 1;
const HEADER_SIZE: usize = 64;
const SEG_SIZE: usize = 40;
const PT_LOAD: u32 = 1;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        eprintln!("usage: gvtcier-gcx <kernel.elf> <out.gcx>");
        std::process::exit(1);
    }
    let data = fs::read(&args[1]).expect("read elf");
    if data.len() < 64 || &data[0..4] != b"\x7fELF" {
        eprintln!("not an ELF file");
        std::process::exit(1);
    }
    let e_entry = u64::from_le_bytes(data[0x18..0x20].try_into().unwrap());
    let e_phoff = u64::from_le_bytes(data[0x20..0x28].try_into().unwrap()) as usize;
    let e_phentsize = u16::from_le_bytes(data[0x36..0x38].try_into().unwrap()) as usize;
    let e_phnum = u16::from_le_bytes(data[0x38..0x3A].try_into().unwrap()) as usize;

    let mut segs: Vec<(u64, u64, u64, u64)> = Vec::new();
    for i in 0..e_phnum {
        let off = e_phoff + i * e_phentsize;
        if off + 56 > data.len() {
            eprintln!("truncated program header");
            std::process::exit(1);
        }
        let p_type = u32::from_le_bytes(data[off..off + 4].try_into().unwrap());
        if p_type != PT_LOAD {
            continue;
        }
        let p_offset = u64::from_le_bytes(data[off + 8..off + 16].try_into().unwrap());
        let p_vaddr = u64::from_le_bytes(data[off + 16..off + 24].try_into().unwrap());
        let p_filesz = u64::from_le_bytes(data[off + 32..off + 40].try_into().unwrap());
        let p_memsz = u64::from_le_bytes(data[off + 40..off + 48].try_into().unwrap());
        segs.push((p_offset, p_vaddr, p_filesz, p_memsz));
    }
    if segs.is_empty() {
        eprintln!("no PT_LOAD segments");
        std::process::exit(1);
    }

    let mut segs_out: Vec<(u64, u64, u64, u64)> = Vec::new();
    let mut blob: Vec<u8> = Vec::new();
    let mut cur: u64 = (HEADER_SIZE + segs.len() * SEG_SIZE) as u64;
    for (p_offset, p_vaddr, p_filesz, p_memsz) in &segs {
        let s = *p_offset as usize;
        let e = (*p_offset + *p_filesz) as usize;
        if e > data.len() {
            eprintln!("bad segment file range");
            std::process::exit(1);
        }
        blob.extend_from_slice(&data[s..e]);
        segs_out.push((cur, *p_vaddr, *p_filesz, *p_memsz));
        cur += *p_filesz;
    }

    let mut out = Vec::new();
    out.extend_from_slice(&GCX_MAGIC.to_le_bytes());
    out.extend_from_slice(&GCX_VERSION.to_le_bytes());
    out.extend_from_slice(&(segs_out.len() as u32).to_le_bytes());
    out.extend_from_slice(&e_entry.to_le_bytes());
    out.extend_from_slice(&KERNEL_VIRT.to_le_bytes());
    out.extend_from_slice(&(HEADER_SIZE as u64).to_le_bytes());
    out.resize(HEADER_SIZE, 0);
    for (off, vaddr, filesz, memsz) in &segs_out {
        out.extend_from_slice(&off.to_le_bytes());
        out.extend_from_slice(&vaddr.to_le_bytes());
        out.extend_from_slice(&filesz.to_le_bytes());
        out.extend_from_slice(&memsz.to_le_bytes());
        out.extend_from_slice(&0u32.to_le_bytes());
        out.extend_from_slice(&0u32.to_le_bytes());
    }
    out.extend_from_slice(&blob);

    fs::write(&args[2], &out).expect("write gcx");
    println!(
        "gcx written: {} bytes, {} segments, entry={:#x}",
        out.len(),
        segs_out.len(),
        e_entry
    );
}
