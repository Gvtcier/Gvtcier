#[no_mangle]
static mut F_RET: u64 = 0;

fn sys_disk_read(lba: u64, count: u64, buf: u64) -> u64 {
    unsafe {
        core::arch::asm!(
            "mov r8, rsp",
            "lea r9, [rip + F_RET]",
            "syscall",
            inout("rax") 9u64 => _,
            in("rdi") lba,
            in("rsi") count,
            in("rdx") buf,
            out("r8") _,
            out("r9") _,
            out("rcx") _,
            out("r11") _,
            options(nostack),
        );
        F_RET
    }
}

fn sys_disk_write(lba: u64, count: u64, buf: u64) -> u64 {
    unsafe {
        core::arch::asm!(
            "mov r8, rsp",
            "lea r9, [rip + F_RET]",
            "syscall",
            inout("rax") 36u64 => _,
            in("rdi") lba,
            in("rsi") count,
            in("rdx") buf,
            out("r8") _,
            out("r9") _,
            out("rcx") _,
            out("r11") _,
            options(nostack),
        );
        F_RET
    }
}

pub const MAX_FILES: usize = 8;

#[repr(C)]
pub struct File {
    pub valid: bool,
    pub first_cluster: u32,
    pub size: u32,
    pub pos: u32,
    pub cluster: u32,
    pub cluster_pos: u32,
}

const FILE_NONE: File = File {
    valid: false,
    first_cluster: 0,
    size: 0,
    pos: 0,
    cluster: 0,
    cluster_pos: 0,
};

static mut FILES: [File; MAX_FILES] = [FILE_NONE; MAX_FILES];

static mut SPC: u32 = 1;
static mut RESERVED: u32 = 1;
static mut FATS: u32 = 2;
static mut ROOT_ENTRIES: u32 = 512;
static mut FAT_SIZE: u32 = 128;
static mut DATA_START: u32 = 0;

pub fn init() {
    let mut bs = [0u8; 512];
    sys_disk_read(0, 1, bs.as_mut_ptr() as u64);
    unsafe {
        SPC = bs[13] as u32;
        RESERVED = u16::from_le_bytes([bs[14], bs[15]]) as u32;
        FATS = bs[16] as u32;
        ROOT_ENTRIES = u16::from_le_bytes([bs[17], bs[18]]) as u32;
        FAT_SIZE = u16::from_le_bytes([bs[22], bs[23]]) as u32;
        let root_sectors = ROOT_ENTRIES * 32 / 512;
        DATA_START = RESERVED + FATS * FAT_SIZE + root_sectors;
    }
}

fn fat_next(cluster: u32) -> u32 {
    let reserved = unsafe { RESERVED };
    let mut buf = [0u8; 512];
    let fat_sector = reserved + cluster * 2 / 512;
    sys_disk_read(fat_sector as u64, 1, buf.as_mut_ptr() as u64);
    let idx = (cluster * 2 % 512) as usize;
    u16::from_le_bytes([buf[idx], buf[idx + 1]]) as u32
}

pub fn fat_open(name: &[u8]) -> u32 {
    let reserved = unsafe { RESERVED };
    let fats = unsafe { FATS };
    let fat_size = unsafe { FAT_SIZE };
    let root_entries = unsafe { ROOT_ENTRIES };
    let root_sectors = root_entries * 32 / 512;
    let root_start = reserved + fats * fat_size;

    let mut fname = [b' '; 11];
    let dot = name.iter().position(|&c| c == b'.').unwrap_or(name.len());
    for (i, &c) in name[..dot].iter().enumerate() {
        if i < 8 {
            fname[i] = c.to_ascii_uppercase();
        }
    }
    if dot < name.len() {
        for (i, &c) in name[dot + 1..].iter().enumerate() {
            if i < 3 {
                fname[8 + i] = c.to_ascii_uppercase();
            }
        }
    }

    for s in 0..root_sectors {
        let mut buf = [0u8; 512];
        sys_disk_read((root_start + s) as u64, 1, buf.as_mut_ptr() as u64);
        for e in 0..16 {
            let off = e * 32;
            if buf[off] == 0 {
                return 0xFFFFFFFF;
            }
            if buf[off] == 0xE5 {
                continue;
            }
            if buf[off + 11] & 0x08 != 0 {
                continue;
            }
            if &buf[off..off + 11] == fname.as_slice() {
                let first_cluster =
                    u16::from_le_bytes([buf[off + 26], buf[off + 27]]) as u32;
                let size = u32::from_le_bytes([
                    buf[off + 28],
                    buf[off + 29],
                    buf[off + 30],
                    buf[off + 31],
                ]);
                unsafe {
                    for i in 0..MAX_FILES {
                        if !FILES[i].valid {
                            FILES[i] = File {
                                valid: true,
                                first_cluster,
                                size,
                                pos: 0,
                                cluster: first_cluster,
                                cluster_pos: 0,
                            };
                            return i as u32;
                        }
                    }
                }
            }
        }
    }
    0xFFFFFFFF
}

pub fn fat_read(handle: u32, buf: &mut [u8]) -> u32 {
    unsafe {
        let f = &mut FILES[handle as usize];
        if !f.valid {
            return 0;
        }
        let spc = SPC;
        let data_start = DATA_START;
        let mut total = 0;
        while total < buf.len() && f.pos < f.size {
            let sector_in_cluster = f.cluster_pos / 512;
            let byte_in_sector = f.cluster_pos % 512;
            let cluster_sector = data_start + (f.cluster - 2) * spc + sector_in_cluster;
            let mut sec = [0u8; 512];
            sys_disk_read(cluster_sector as u64, 1, sec.as_mut_ptr() as u64);
            let n = core::cmp::min(
                512 - byte_in_sector as usize,
                core::cmp::min(buf.len() - total, (f.size - f.pos) as usize),
            );
            buf[total..total + n]
                .copy_from_slice(&sec[byte_in_sector as usize..byte_in_sector as usize + n]);
            total += n;
            f.pos += n as u32;
            f.cluster_pos += n as u32;
            if f.cluster_pos >= spc * 512 {
                f.cluster_pos = 0;
                f.cluster = fat_next(f.cluster);
            }
        }
        total as u32
    }
}

pub fn fat_close(handle: u32) {
    unsafe {
        if (handle as usize) < MAX_FILES {
            FILES[handle as usize] = FILE_NONE;
        }
    }
}

fn fat_set(cluster: u32, value: u32) {
    let reserved = unsafe { RESERVED };
    let mut buf = [0u8; 512];
    let fat_sector = reserved + cluster * 2 / 512;
    sys_disk_read(fat_sector as u64, 1, buf.as_mut_ptr() as u64);
    let idx = (cluster * 2 % 512) as usize;
    buf[idx] = (value & 0xFF) as u8;
    buf[idx + 1] = ((value >> 8) & 0xFF) as u8;
    sys_disk_write(fat_sector as u64, 1, buf.as_mut_ptr() as u64);
}

fn alloc_cluster() -> u32 {
    let reserved = unsafe { RESERVED };
    let fat_size = unsafe { FAT_SIZE };
    for cluster in 2..fat_size * 256 {
        let mut buf = [0u8; 512];
        let fat_sector = reserved + cluster * 2 / 512;
        sys_disk_read(fat_sector as u64, 1, buf.as_mut_ptr() as u64);
        let idx = (cluster * 2 % 512) as usize;
        let v = u16::from_le_bytes([buf[idx], buf[idx + 1]]) as u32;
        if v == 0 {
            return cluster;
        }
    }
    0
}

pub fn fat_write(handle: u32, buf: &[u8]) -> u32 {
    unsafe {
        let f = &mut FILES[handle as usize];
        if !f.valid {
            return 0;
        }
        let spc = SPC;
        let data_start = DATA_START;
        let mut total = 0;
        while total < buf.len() {
            let sector_in_cluster = f.cluster_pos / 512;
            let byte_in_sector = f.cluster_pos % 512;
            let cluster_sector = data_start + (f.cluster - 2) * spc + sector_in_cluster;
            let mut sec = [0u8; 512];
            sys_disk_read(cluster_sector as u64, 1, sec.as_mut_ptr() as u64);
            let n = core::cmp::min(512 - byte_in_sector as usize, buf.len() - total);
            sec[byte_in_sector as usize..byte_in_sector as usize + n]
                .copy_from_slice(&buf[total..total + n]);
            sys_disk_write(cluster_sector as u64, 1, sec.as_mut_ptr() as u64);
            total += n;
            f.pos += n as u32;
            f.cluster_pos += n as u32;
            if f.cluster_pos >= spc * 512 {
                f.cluster_pos = 0;
                let next = alloc_cluster();
                if next == 0 {
                    break;
                }
                fat_set(f.cluster, next);
                fat_set(next, 0xFFF);
                f.cluster = next;
            }
        }
        if f.pos > f.size {
            f.size = f.pos;
        }
        total as u32
    }
}
