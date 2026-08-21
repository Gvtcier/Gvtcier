use std::env;
use std::fs;

const SECTOR: usize = 2048;
const LBA_PVD: u32 = 16;
const LBA_BOOT_RECORD: u32 = 17;
const LBA_TERMINATOR: u32 = 18;
const LBA_BOOT_CATALOG: u32 = 19;
const LBA_FAT: u32 = 32;
const DIR_SIZE: u32 = 2048;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 5 {
        eprintln!("usage: gvtcier-iso <bootx64.efi> <kernel.elf> <out.iso> <uefi|bios>");
        std::process::exit(1);
    }
    let mode = args[4].as_str();
    let uefi_default = mode == "uefi";
    let bootx64 = fs::read(&args[1]).expect("read bootx64");
    let kernel = fs::read(&args[2]).expect("read kernel");
    println!("read kernel: {} bytes", kernel.len());

    let fat12_img = make_fat12(&bootx64, &kernel);
    let fat32_img = make_fat32(&bootx64, &kernel);

    let fat12_sectors = div_ceil(fat12_img.len(), SECTOR);
    let lba_fat32 = LBA_FAT + fat12_sectors as u32;
    let fat32_sectors = div_ceil(fat32_img.len(), SECTOR);
    let lba_root_dir = lba_fat32 + fat32_sectors as u32;
    let total_sectors = lba_root_dir + 1;

    let root_dir = build_dir(vec![
        dir_record(b"\x00", lba_root_dir, DIR_SIZE, true),
        dir_record(b"FAT.IMG", LBA_FAT, fat12_img.len() as u32, false),
        dir_record(b"FAT32.IMG", lba_fat32, fat32_img.len() as u32, false),
    ]);

    let mut img = vec![0u8; total_sectors as usize * SECTOR];
    img[lba_off(LBA_FAT)..lba_off(LBA_FAT) + fat12_img.len()].copy_from_slice(&fat12_img);
    img[lba_off(lba_fat32)..lba_off(lba_fat32) + fat32_img.len()].copy_from_slice(&fat32_img);
    img[lba_off(lba_root_dir)..lba_off(lba_root_dir) + root_dir.len()].copy_from_slice(&root_dir);

    write_pvd(&mut img, total_sectors, lba_root_dir, DIR_SIZE as usize);
    write_boot_record(&mut img);
    write_terminator(&mut img);
    write_boot_catalog(&mut img, fat12_img.len(), fat32_img.len(), lba_fat32, uefi_default);

    fs::write(&args[3], &img).expect("write iso");
    println!(
        "iso written: {} sectors ({} bytes), fat12 {} bytes, fat32 {} bytes, boot={}",
        total_sectors,
        img.len(),
        fat12_img.len(),
        fat32_img.len(),
        if uefi_default { "uefi" } else { "bios" }
    );
}

fn make_fat12(bootx64: &[u8], kernel: &[u8]) -> Vec<u8> {
    const BS: usize = 512;
    const FAT_SECTORS: usize = 9;
    const ROOT_ENTRIES: usize = 224;
    const ROOT_SECTORS: usize = 14;
    const STAGE2_SECTORS: usize = 64;
    const TOTAL: usize = 2880;
    const RESERVED: usize = 1 + STAGE2_SECTORS;
    const FAT_START: usize = RESERVED;
    const ROOT_START: usize = FAT_START + 2 * FAT_SECTORS;
    const DATA_START: usize = ROOT_START + ROOT_SECTORS;

    let manifest = env!("CARGO_MANIFEST_DIR");
    let repo_root = std::path::Path::new(manifest)
        .parent()
        .expect("repo root");
    let boot_bin = std::fs::read(repo_root.join("BIOS/boot.bin")).expect("read BIOS/boot.bin");
    let stage2 = std::fs::read(repo_root.join("BIOS/stage2.bin")).expect("read BIOS/stage2.bin");

    let bootx64_clusters = div_ceil(bootx64.len(), BS);
    let kernel_clusters = div_ceil(kernel.len(), BS);
    let kernel_start = 2 + bootx64_clusters;
    let efi_cluster = kernel_start + kernel_clusters;
    let boot_cluster = efi_cluster + 1;

    let mut img = vec![0u8; TOTAL * BS];
    img[0..512].copy_from_slice(&boot_bin[..512]);
    img[BS..BS + stage2.len()].copy_from_slice(&stage2);
    img[3..11].copy_from_slice(b"GVTCIER ");
    put_u16_le(&mut img, 11, 512);
    img[13] = 1;
    put_u16_le(&mut img, 14, RESERVED as u16);
    img[16] = 2;
    put_u16_le(&mut img, 17, ROOT_ENTRIES as u16);
    put_u16_le(&mut img, 19, TOTAL as u16);
    img[21] = 0xF0;
    put_u16_le(&mut img, 22, FAT_SECTORS as u16);
    put_u16_le(&mut img, 24, 18);
    put_u16_le(&mut img, 26, 2);
    put_u32_le(&mut img, 28, 0);
    put_u32_le(&mut img, 32, 0);
    img[36] = 0;
    img[37] = 0;
    img[38] = 0x29;
    put_u32_le(&mut img, 39, 0x47565443);
    img[43..54].copy_from_slice(b"GVTCIER    ");
    img[54..62].copy_from_slice(b"FAT12   ");
    img[510] = 0x55;
    img[511] = 0xAA;

    let mut fat = vec![0u8; FAT_SECTORS * BS];
    set_fat12(&mut fat, 0, 0xFF0);
    set_fat12(&mut fat, 1, 0xFFF);
    for i in 0..bootx64_clusters {
        let c = 2 + i;
        let next = if i + 1 < bootx64_clusters { c + 1 } else { 0xFFF };
        set_fat12(&mut fat, c, next as u16);
    }
    for i in 0..kernel_clusters {
        let c = kernel_start + i;
        let next = if i + 1 < kernel_clusters { c + 1 } else { 0xFFF };
        set_fat12(&mut fat, c, next as u16);
    }
    set_fat12(&mut fat, efi_cluster, 0xFFF);
    set_fat12(&mut fat, boot_cluster, 0xFFF);
    img[FAT_START * BS..FAT_START * BS + fat.len()].copy_from_slice(&fat);
    img[(FAT_START + FAT_SECTORS) * BS..(FAT_START + 2 * FAT_SECTORS) * BS]
        .copy_from_slice(&fat);

    img[DATA_START * BS..DATA_START * BS + bootx64.len()].copy_from_slice(bootx64);
    let ko = (DATA_START + kernel_start - 2) * BS;
    img[ko..ko + kernel.len()].copy_from_slice(kernel);
    let eo = (DATA_START + efi_cluster - 2) * BS;
    let bo = (DATA_START + boot_cluster - 2) * BS;

    let e = fat_dir_entry(*b".       ", *b"   ", 0x10, efi_cluster as u16, 0);
    img[eo..eo + 32].copy_from_slice(&e);
    let e = fat_dir_entry(*b"..      ", *b"   ", 0x10, 2, 0);
    img[eo + 32..eo + 64].copy_from_slice(&e);
    let e = fat_dir_entry(*b"BOOT    ", *b"   ", 0x10, boot_cluster as u16, 0);
    img[eo + 64..eo + 96].copy_from_slice(&e);
    let e = fat_dir_entry(*b".       ", *b"   ", 0x10, boot_cluster as u16, 0);
    img[bo..bo + 32].copy_from_slice(&e);
    let e = fat_dir_entry(*b"..      ", *b"   ", 0x10, efi_cluster as u16, 0);
    img[bo + 32..bo + 64].copy_from_slice(&e);
    let e = fat_dir_entry(*b"BOOTX64 ", *b"EFI", 0x20, 2, bootx64.len() as u32);
    img[bo + 64..bo + 96].copy_from_slice(&e);

    let root_off = ROOT_START * BS;
    let mut entry_off = root_off;
    let e = fat_dir_entry(*b"EFI     ", *b"   ", 0x10, efi_cluster as u16, 0);
    img[entry_off..entry_off + 32].copy_from_slice(&e);
    entry_off += 32;
    let e = fat_dir_entry(*b"KERNEL  ", *b"GCX", 0x20, kernel_start as u16, kernel.len() as u32);
    img[entry_off..entry_off + 32].copy_from_slice(&e);

    img
}

fn make_fat32(bootx64: &[u8], kernel: &[u8]) -> Vec<u8> {
    const BS: usize = 512;
    const TOTAL: usize = 66816;
    const RSVD: usize = 32;
    const FAT_SECTORS: usize = 512;
    const NUM_FATS: usize = 2;
    const DATA_START: usize = RSVD + NUM_FATS * FAT_SECTORS;
    const ROOT_CLUSTER: usize = 2;
    const EFI_CLUSTER: usize = 3;
    const BOOT_DIR_CLUSTER: usize = 4;
    const BOOT_START: usize = 5;

    let boot_clusters = div_ceil(bootx64.len(), BS);
    let kernel_start = BOOT_START + boot_clusters;
    let kernel_clusters = div_ceil(kernel.len(), BS);

    let mut img = vec![0u8; TOTAL * BS];
    img[0] = 0xEB;
    img[1] = 0x58;
    img[2] = 0x90;
    img[3..11].copy_from_slice(b"GVTCIER ");
    put_u16_le(&mut img, 11, 512);
    img[13] = 1;
    put_u16_le(&mut img, 14, RSVD as u16);
    img[16] = NUM_FATS as u8;
    put_u16_le(&mut img, 17, 0);
    put_u16_le(&mut img, 19, 0);
    img[21] = 0xF8;
    put_u16_le(&mut img, 22, 0);
    put_u16_le(&mut img, 24, 63);
    put_u16_le(&mut img, 26, 255);
    put_u32_le(&mut img, 28, 0);
    put_u32_le(&mut img, 32, TOTAL as u32);
    put_u32_le(&mut img, 36, FAT_SECTORS as u32);
    put_u16_le(&mut img, 40, 0);
    put_u16_le(&mut img, 42, 0);
    put_u32_le(&mut img, 44, ROOT_CLUSTER as u32);
    put_u16_le(&mut img, 48, 1);
    put_u16_le(&mut img, 50, 6);
    img[64] = 0x80;
    img[65] = 0;
    img[66] = 0x29;
    put_u32_le(&mut img, 67, 0x47565443);
    img[71..82].copy_from_slice(b"GVTCIER    ");
    img[82..90].copy_from_slice(b"FAT32   ");
    img[510] = 0x55;
    img[511] = 0xAA;

    let fs = BS;
    put_u32_le(&mut img, fs, 0x41615252);
    put_u32_le(&mut img, fs + 484, 0x61417272);
    put_u32_le(&mut img, fs + 488, 0xFFFFFFFF);
    put_u32_le(&mut img, fs + 492, ROOT_CLUSTER as u32);
    put_u32_le(&mut img, fs + 508, 0xAA550000);

    let mut fat = vec![0u8; FAT_SECTORS * BS];
    set_fat32(&mut fat, 0, 0x0FFFFFF8);
    set_fat32(&mut fat, 1, 0xFFFFFFFF);
    set_fat32(&mut fat, ROOT_CLUSTER, 0x0FFFFFFF);
    set_fat32(&mut fat, EFI_CLUSTER, 0x0FFFFFFF);
    set_fat32(&mut fat, BOOT_DIR_CLUSTER, 0x0FFFFFFF);
    for i in 0..boot_clusters {
        let c = BOOT_START + i;
        let next = if i + 1 < boot_clusters { c + 1 } else { 0x0FFFFFFF };
        set_fat32(&mut fat, c, next as u32);
    }
    for i in 0..kernel_clusters {
        let c = kernel_start + i;
        let next = if i + 1 < kernel_clusters { c + 1 } else { 0x0FFFFFFF };
        set_fat32(&mut fat, c, next as u32);
    }
    for i in 0..NUM_FATS {
        let off = (RSVD + i * FAT_SECTORS) * BS;
        img[off..off + fat.len()].copy_from_slice(&fat);
    }

    let cluster_off = |c: usize| (DATA_START + (c - 2)) * BS;
    let mut root = [0u8; BS];
    root[0..32].copy_from_slice(&fat_dir_entry(*b"EFI     ", *b"   ", 0x10, EFI_CLUSTER as u16, 0));
    root[32..64].copy_from_slice(&fat_dir_entry(*b"KERNEL  ", *b"GCX", 0x20, kernel_start as u16, kernel.len() as u32));
    img[cluster_off(ROOT_CLUSTER)..cluster_off(ROOT_CLUSTER) + BS].copy_from_slice(&root);
    let mut efi_dir = [0u8; BS];
    efi_dir[0..32].copy_from_slice(&fat_dir_entry(*b".       ", *b"   ", 0x10, EFI_CLUSTER as u16, 0));
    efi_dir[32..64].copy_from_slice(&fat_dir_entry(*b"..      ", *b"   ", 0x10, ROOT_CLUSTER as u16, 0));
    efi_dir[64..96].copy_from_slice(&fat_dir_entry(*b"BOOT    ", *b"   ", 0x10, BOOT_DIR_CLUSTER as u16, 0));
    img[cluster_off(EFI_CLUSTER)..cluster_off(EFI_CLUSTER) + BS].copy_from_slice(&efi_dir);
    let mut boot_dir = [0u8; BS];
    boot_dir[0..32].copy_from_slice(&fat_dir_entry(*b".       ", *b"   ", 0x10, BOOT_DIR_CLUSTER as u16, 0));
    boot_dir[32..64].copy_from_slice(&fat_dir_entry(*b"..      ", *b"   ", 0x10, EFI_CLUSTER as u16, 0));
    boot_dir[64..96].copy_from_slice(&fat_dir_entry(*b"BOOTX64 ", *b"EFI", 0x20, BOOT_START as u16, bootx64.len() as u32));
    img[cluster_off(BOOT_DIR_CLUSTER)..cluster_off(BOOT_DIR_CLUSTER) + BS].copy_from_slice(&boot_dir);
    img[cluster_off(BOOT_START)..cluster_off(BOOT_START) + bootx64.len()].copy_from_slice(bootx64);
    img[cluster_off(kernel_start)..cluster_off(kernel_start) + kernel.len()].copy_from_slice(kernel);

    img
}

fn set_fat32(fat: &mut [u8], c: usize, v: u32) {
    put_u32_le(fat, c * 4, v);
}

fn fat_dir_entry(name: [u8; 8], ext: [u8; 3], attr: u8, cluster: u16, size: u32) -> [u8; 32] {
    let mut e = [0u8; 32];
    e[0..8].copy_from_slice(&name);
    e[8..11].copy_from_slice(&ext);
    e[11] = attr;
    put_u16_le(&mut e, 14, 0);
    put_u16_le(&mut e, 16, 0x5D0E);
    put_u16_le(&mut e, 18, 0x5D0E);
    put_u16_le(&mut e, 20, 0);
    put_u16_le(&mut e, 22, 0);
    put_u16_le(&mut e, 24, 0x5D0E);
    put_u16_le(&mut e, 26, cluster);
    put_u32_le(&mut e, 28, size);
    e
}

fn set_fat12(fat: &mut [u8], c: usize, v: u16) {
    let off = c + c / 2;
    if c % 2 == 0 {
        fat[off] = (v & 0xFF) as u8;
        fat[off + 1] = (fat[off + 1] & 0xF0) | ((v >> 8) & 0x0F) as u8;
    } else {
        fat[off] = (fat[off] & 0x0F) | (((v & 0x0F) as u8) << 4);
        fat[off + 1] = ((v >> 4) & 0xFF) as u8;
    }
}

fn write_pvd(img: &mut [u8], total_sectors: u32, root_lba: u32, root_size: usize) {
    let s = lba_off(LBA_PVD);
    img[s] = 1;
    img[s + 1..s + 6].copy_from_slice(b"CD001");
    img[s + 6] = 1;
    write_fixed(&mut img[s + 7..s + 39], b"GVTCIER OS");
    write_fixed(&mut img[s + 40..s + 72], b"GVTCIER");
    put_u32_le(img, s + 80, total_sectors);
    put_u32_be(img, s + 84, total_sectors);
    put_u16_le(img, s + 120, 1);
    put_u16_be(img, s + 122, 1);
    put_u16_le(img, s + 124, 1);
    put_u16_be(img, s + 126, 1);
    put_u16_le(img, s + 128, 2048);
    put_u16_be(img, s + 130, 2048);
    put_root_record(img, s + 156, root_lba, root_size);
}

fn put_root_record(img: &mut [u8], off: usize, extent: u32, size: usize) {
    img[off] = 34;
    img[off + 1] = 0;
    put_u32_le(img, off + 2, extent);
    put_u32_be(img, off + 6, extent);
    put_u32_le(img, off + 10, size as u32);
    put_u32_be(img, off + 14, size as u32);
    img[off + 18] = 0;
    img[off + 19] = 1;
    img[off + 20] = 1;
    img[off + 21] = 0;
    img[off + 22] = 0;
    img[off + 23] = 0;
    img[off + 24] = 0;
    img[off + 25] = 2;
    img[off + 26] = 0;
    img[off + 27] = 0;
    put_u16_le(img, off + 28, 1);
    put_u16_be(img, off + 30, 1);
    img[off + 32] = 1;
    img[off + 33] = 0;
}

fn write_boot_record(img: &mut [u8]) {
    let s = lba_off(LBA_BOOT_RECORD);
    img[s] = 0;
    img[s + 1..s + 6].copy_from_slice(b"CD001");
    img[s + 6] = 1;
    write_fixed(&mut img[s + 7..s + 39], b"EL TORITO SPECIFICATION");
    put_u32_le(img, s + 71, LBA_BOOT_CATALOG);
}

fn write_terminator(img: &mut [u8]) {
    let s = lba_off(LBA_TERMINATOR);
    img[s] = 255;
    img[s + 1..s + 6].copy_from_slice(b"CD001");
    img[s + 6] = 1;
}

fn write_boot_catalog(img: &mut [u8], fat12_size: usize, fat32_size: usize, lba_fat32: u32, uefi_default: bool) {
    let s = lba_off(LBA_BOOT_CATALOG);
    img[s] = 0x01;
    img[s + 1] = 0x00;
    img[s + 2..s + 4].fill(0);
    write_fixed(&mut img[s + 4..s + 28], b"GVTCIER");
    img[s + 30] = 0x55;
    img[s + 31] = 0xAA;
    let mut sum: u32 = 0;
    for i in 0..16 {
        sum += u16::from_le_bytes([img[s + i * 2], img[s + i * 2 + 1]]) as u32;
    }
    let checksum = (0x10000u32 - sum % 0x10000) % 0x10000;
    put_u16_le(img, s + 28, checksum as u16);
    if uefi_default {
        // UEFI 默认:0xEF 记录在前,0x88 记录随后
        img[s + 32] = 0xEF;
        img[s + 33] = 0x01;
        put_u16_le(img, s + 34, 0);
        img[s + 36] = 0;
        img[s + 37] = 0;
        put_u16_le(img, s + 38, div_ceil(fat12_size, SECTOR) as u16);
        put_u32_le(img, s + 40, LBA_FAT);
        img[s + 64] = 0x88;
        img[s + 65] = 0x02;
        put_u16_le(img, s + 66, 0);
        img[s + 68] = 0;
        img[s + 69] = 0;
        put_u16_le(img, s + 70, div_ceil(fat32_size, SECTOR) as u16);
        put_u32_le(img, s + 72, lba_fat32);
    } else {
        // BIOS 默认:0x88 记录在前
        img[s + 32] = 0x88;
        img[s + 33] = 0x02;
        put_u16_le(img, s + 34, 0);
        img[s + 36] = 0;
        img[s + 37] = 0;
        put_u16_le(img, s + 38, div_ceil(fat12_size, SECTOR) as u16);
        put_u32_le(img, s + 40, LBA_FAT);
        img[s + 64] = 0x88;
        img[s + 65] = 0x00;
        put_u16_le(img, s + 66, 0);
        img[s + 68] = 0;
        img[s + 69] = 0;
        put_u16_le(img, s + 70, div_ceil(fat32_size, SECTOR) as u16);
        put_u32_le(img, s + 72, lba_fat32);
    }
}

fn build_dir(records: Vec<Vec<u8>>) -> Vec<u8> {
    let mut d = Vec::new();
    for r in records {
        d.extend_from_slice(&r);
    }
    d.push(0);
    while d.len() % SECTOR != 0 {
        d.push(0);
    }
    d
}

fn dir_record(name: &[u8], extent: u32, size: u32, is_dir: bool) -> Vec<u8> {
    let name_len = name.len();
    let padded = 34 + name_len + ((34 + name_len) & 1);
    let mut r = vec![0u8; padded];
    r[0] = padded as u8;
    put_u32_le(&mut r, 2, extent);
    put_u32_be(&mut r, 6, extent);
    put_u32_le(&mut r, 10, size);
    put_u32_be(&mut r, 14, size);
    r[18] = 0;
    r[19] = 1;
    r[20] = 1;
    r[25] = if is_dir { 2 } else { 0 };
    put_u16_le(&mut r, 28, 1);
    put_u16_be(&mut r, 30, 1);
    r[32] = name_len as u8;
    r[33..33 + name_len].copy_from_slice(name);
    r
}

fn write_fixed(dst: &mut [u8], src: &[u8]) {
    let n = src.len().min(dst.len());
    dst[..n].copy_from_slice(&src[..n]);
    dst[n..].fill(0);
}

fn put_u16_le(b: &mut [u8], off: usize, v: u16) {
    b[off] = v as u8;
    b[off + 1] = (v >> 8) as u8;
}

fn put_u16_be(b: &mut [u8], off: usize, v: u16) {
    b[off] = (v >> 8) as u8;
    b[off + 1] = v as u8;
}

fn put_u32_le(b: &mut [u8], off: usize, v: u32) {
    b[off] = v as u8;
    b[off + 1] = (v >> 8) as u8;
    b[off + 2] = (v >> 16) as u8;
    b[off + 3] = (v >> 24) as u8;
}

fn put_u32_be(b: &mut [u8], off: usize, v: u32) {
    b[off] = (v >> 24) as u8;
    b[off + 1] = (v >> 16) as u8;
    b[off + 2] = (v >> 8) as u8;
    b[off + 3] = v as u8;
}

fn div_ceil(a: usize, b: usize) -> usize {
    (a + b - 1) / b
}

fn lba_off(lba: u32) -> usize {
    lba as usize * SECTOR
}
