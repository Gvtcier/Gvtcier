use crate::io::Ahci;

const MAX_FILES: usize = 8;
const FAT_EOF: u32 = 0x0FFFFFF8;
const BITMAP_CLUSTER: u32 = 3;
const DIR_ATTR: u8 = 0x10;
const FILE_ATTR: u8 = 0x20;
static mut FAT_START: u64 = 12;
static mut ROOT_CLUSTER: u32 = 2;
static mut CLUSTER_COUNT: u32 = 4090;

static mut FILE_TABLE: [u32; MAX_FILES] = [0; MAX_FILES];
static mut FILE_SIZE: [u32; MAX_FILES] = [0; MAX_FILES];
static mut HEAP: u64 = 0;
static mut SPC: u32 = 0;
static mut CWD: u32 = 2;
static mut BUF: [u8; 512 * 64] = [0; 512 * 64];
static mut FAT_BUF: [u8; 512] = [0; 512];
static mut FAT_SECTOR: u64 = 0xFFFFFFFF;
static mut CLUSTER_BUF: [u8; 512 * 8] = [0; 512 * 8];
static mut SECTOR: [u8; 512] = [0; 512];

fn rd32(b: &[u8], off: usize) -> u32 {
    u32::from_le_bytes([b[off], b[off + 1], b[off + 2], b[off + 3]])
}

fn dir_match_name(e: &[u8], name: &[u8]) -> bool {
    let nlen = u16::from_le_bytes([e[2], e[3]]) as usize;
    if nlen != name.len() {
        return false;
    }
    for k in 0..nlen {
        let ch = (e[4 + k * 2] as u16) | ((e[5 + k * 2] as u16) << 8);
        if ch > 0x7F || (ch as u8) != name[k] {
            return false;
        }
    }
    true
}

fn fat_next(cluster: u32) -> u32 {
    unsafe {
        let fs = FAT_START + (cluster as u64 * 4) / 512;
        if FAT_SECTOR != fs {
            if Ahci::read(fs, 1, &mut FAT_BUF) != 0 {
                return FAT_EOF;
            }
            FAT_SECTOR = fs;
        }
        rd32(&FAT_BUF, ((cluster * 4) % 512) as usize)
    }
}

fn fat_set(cluster: u32, value: u32) -> u32 {
    unsafe {
        let fs = FAT_START + (cluster as u64 * 4) / 512;
        if Ahci::read(fs, 1, &mut FAT_BUF) != 0 {
            return 1;
        }
        let off = ((cluster * 4) % 512) as usize;
        FAT_BUF[off..off + 4].copy_from_slice(&value.to_le_bytes());
        if Ahci::write_sector(fs, &FAT_BUF) != 0 {
            return 1;
        }
        FAT_SECTOR = 0xFFFFFFFF;
    }
    0
}

fn bitmap_get(cluster: u32) -> bool {
    unsafe {
        let bm_lba = HEAP + (BITMAP_CLUSTER as u64 - ROOT_CLUSTER as u64) * SPC as u64;
        let sec = cluster / 4096;
        if Ahci::read(bm_lba + sec as u64, 1, &mut SECTOR) != 0 {
            return true;
        }
        let bit = cluster % 4096;
        SECTOR[(bit / 8) as usize] >> (bit % 8) & 1 != 0
    }
}

fn bitmap_set(cluster: u32, used: bool) -> u32 {
    unsafe {
        let bm_lba = HEAP + (BITMAP_CLUSTER as u64 - ROOT_CLUSTER as u64) * SPC as u64;
        let sec = cluster / 4096;
        if Ahci::read(bm_lba + sec as u64, 1, &mut SECTOR) != 0 {
            return 1;
        }
        let bit = cluster % 4096;
        let byte = (bit / 8) as usize;
        if used {
            SECTOR[byte] |= 1 << (bit % 8);
        } else {
            SECTOR[byte] &= !(1 << (bit % 8));
        }
        if Ahci::write_sector(bm_lba + sec as u64, &SECTOR) != 0 {
            return 1;
        }
    }
    0
}

fn find_free_cluster() -> u32 {
    unsafe {
        let mut c = 4u32;
        loop {
            if !bitmap_get(c) {
                return c;
            }
            c += 1;
            if c >= CLUSTER_COUNT {
                return 0;
            }
        }
    }
}

fn root_nsect() -> u32 {
    unsafe {
        if SPC > 64 {
            64
        } else {
            SPC
        }
    }
}

fn root_entries() -> usize {
    (root_nsect() as usize) * 16
}

fn read_dir() -> u32 {
    unsafe {
        let lba = HEAP + (CWD as u64 - ROOT_CLUSTER as u64) * SPC as u64;
        Ahci::read(lba, root_nsect(), &mut BUF)
    }
}

fn entry_index(name: &[u8]) -> Option<usize> {
    unsafe {
        let entries = root_entries();
        for i in 0..entries {
            let e = &BUF[i * 32..i * 32 + 32];
            match e[0] {
                0xC1 => {
                    if dir_match_name(e, name) {
                        return Some(i);
                    }
                }
                0x00 => break,
                _ => {}
            }
        }
    }
    None
}

pub fn mount() -> u32 {
    unsafe {
        if Ahci::read(0, 12, &mut BUF) != 0 {
            return 1;
        }
        if &BUF[3..11] != b"EXFAT   " {
            return 1;
        }
        HEAP = rd32(&BUF, 35) as u64;
        SPC = 1u32 << BUF[56];
        FAT_START = rd32(&BUF, 27) as u64;
        CLUSTER_COUNT = rd32(&BUF, 39);
        ROOT_CLUSTER = rd32(&BUF, 43);
    }
    0
}

pub fn open(name: &[u8]) -> u32 {
    unsafe {
        if name.len() == 0 || name.len() > 8 {
            return 0xFFFFFFFF;
        }
        if mount() != 0 {
            return 0xFFFFFFFF;
        }
        if read_dir() != 0 {
            return 0xFFFFFFFF;
        }
        let idx = match entry_index(name) {
            Some(i) => i,
            None => return 0xFFFFFFFF,
        };
        if idx < 2 {
            return 0xFFFFFFFF;
        }
        let c0 = &BUF[(idx - 1) * 32..(idx - 1) * 32 + 32];
        if c0[0] != 0xC0 {
            return 0xFFFFFFFF;
        }
        let first_cluster = rd32(c0, 21);
        let mut dl = [0u8; 8];
        dl[0..7].copy_from_slice(&c0[25..32]);
        let data_len = u64::from_le_bytes(dl);
        if first_cluster == 0 {
            return 0xFFFFFFFF;
        }
        for j in 0..MAX_FILES {
            if FILE_TABLE[j] == 0 {
                FILE_TABLE[j] = first_cluster;
                FILE_SIZE[j] = data_len as u32;
                return j as u32 + 1;
            }
        }
    }
    0xFFFFFFFF
}

pub fn read(handle: u32, buf: *mut u8, max: usize) -> u32 {
    unsafe {
        let idx = handle as usize - 1;
        if idx >= MAX_FILES || FILE_TABLE[idx] == 0 {
            return 0;
        }
        let mut cluster = FILE_TABLE[idx];
        let size = FILE_SIZE[idx] as usize;
        let total = if size < max { size } else { max };
        let mut done = 0usize;
        while done < total && cluster >= ROOT_CLUSTER && cluster < FAT_EOF {
            let lba = HEAP + (cluster as u64 - ROOT_CLUSTER as u64) * SPC as u64;
            let mut off = 0u32;
            while off < SPC && done < total {
                let nsec = if SPC - off > 8 {
                    8
                } else {
                    SPC - off
                };
                if Ahci::read(lba + off as u64, nsec, &mut CLUSTER_BUF) != 0 {
                    return done as u32;
                }
                let n = core::cmp::min(total - done, (nsec as usize) * 512);
                core::ptr::copy_nonoverlapping(CLUSTER_BUF.as_ptr(), buf.add(done), n);
                done += n;
                off += nsec;
                if n < (nsec as usize) * 512 {
                    break;
                }
            }
            cluster = fat_next(cluster);
        }
        done as u32
    }
}

pub fn close(handle: u32) {
    unsafe {
        let idx = handle as usize - 1;
        if idx < MAX_FILES {
            FILE_TABLE[idx] = 0;
            FILE_SIZE[idx] = 0;
        }
    }
}

pub fn write(handle: u32, buf: *const u8, len: usize) -> u32 {
    unsafe {
        let idx = handle as usize - 1;
        if idx >= MAX_FILES || FILE_TABLE[idx] == 0 {
            return 0;
        }
        let cluster = FILE_TABLE[idx];
        let lba = HEAP + (cluster as u64 - ROOT_CLUSTER as u64) * SPC as u64;
        let mut sector = [0u8; 512];
        let n = if len < 512 { len } else { 512 };
        core::ptr::copy_nonoverlapping(buf, sector.as_mut_ptr(), n);
        if crate::io::Ahci::write_sector(lba, &sector) != 0 {
            return 0;
        }
        FILE_SIZE[idx] = n as u32;
        n as u32
    }
}

pub fn list(buf: *mut u8, max: usize) -> u32 {
    unsafe {
        if mount() != 0 {
            return 0;
        }
        if read_dir() != 0 {
            return 0;
        }
        let entries = root_entries();
        let mut out = 0usize;
        for i in 0..entries {
            let e = &BUF[i * 32..i * 32 + 32];
            match e[0] {
                0xC1 => {
                    let nlen = u16::from_le_bytes([e[2], e[3]]) as usize;
                    if out + nlen + 1 > max {
                        break;
                    }
                    for k in 0..nlen {
                        let ch = (e[4 + k * 2] as u16) | ((e[5 + k * 2] as u16) << 8);
                        if ch > 0x7F {
                            break;
                        }
                        *buf.add(out) = ch as u8;
                        out += 1;
                    }
                    *buf.add(out) = b' ';
                    out += 1;
                }
                0x00 => break,
                _ => {}
            }
        }
        out as u32
    }
}

pub fn remove(name: &[u8]) -> u32 {
    unsafe {
        if name.len() == 0 || name.len() > 8 {
            return 1;
        }
        if mount() != 0 {
            return 1;
        }
        if read_dir() != 0 {
            return 1;
        }
        let idx = match entry_index(name) {
            Some(i) => i,
            None => return 1,
        };
        if idx < 2 {
            return 1;
        }
        let c0 = &BUF[(idx - 1) * 32..(idx - 1) * 32 + 32];
        if c0[0] != 0xC0 {
            return 1;
        }
        let mut cluster = rd32(c0, 21);
        while cluster >= 2 && cluster < FAT_EOF {
            let next = fat_next(cluster);
            bitmap_set(cluster, false);
            fat_set(cluster, 0);
            cluster = next;
        }
        let e83 = &mut BUF[(idx - 2) * 32..(idx - 2) * 32 + 32];
        e83.fill(0xE5);
        BUF[(idx - 1) * 32..(idx - 1) * 32 + 32].fill(0xE5);
        BUF[idx * 32..idx * 32 + 32].fill(0xE5);
        let lba = HEAP + (CWD as u64 - ROOT_CLUSTER as u64) * SPC as u64;
        for s in 0..root_nsect() {
            let off = (s as usize) * 512;
            if Ahci::write_sector(lba + s as u64, &BUF[off..off + 512].try_into().unwrap()) != 0 {
                return 1;
            }
        }
    }
    0
}

pub fn mkdir(name: &[u8]) -> u32 {
    unsafe {
        if name.len() == 0 || name.len() > 8 {
            return 1;
        }
        if mount() != 0 {
            return 1;
        }
        if read_dir() != 0 {
            return 1;
        }
        if entry_index(name).is_some() {
            return 1;
        }
        let cluster = find_free_cluster();
        if cluster == 0 {
            return 1;
        }
        if bitmap_set(cluster, true) != 0 || fat_set(cluster, FAT_EOF) != 0 {
            return 1;
        }
        let lba = HEAP + (cluster as u64 - ROOT_CLUSTER as u64) * SPC as u64;
        if Ahci::write_sector(lba, &[0u8; 512]) != 0 {
            return 1;
        }
        let entries = root_entries();
        let mut slot = usize::MAX;
        for i in 0..entries {
            if BUF[i * 32] == 0 || BUF[i * 32] == 0xE5 {
                slot = i;
                break;
            }
        }
        if slot == usize::MAX {
            return 1;
        }
        let mut e = [0u8; 32];
        e[0] = 0x83;
        e[3] = DIR_ATTR;
        BUF[slot * 32..slot * 32 + 32].copy_from_slice(&e);
        e = [0u8; 32];
        e[0] = 0xC0;
        e[1] = 0x01;
        e[4] = name.len() as u8;
        e[21..25].copy_from_slice(&cluster.to_le_bytes());
        BUF[slot * 32 + 32..slot * 32 + 64].copy_from_slice(&e);
        e = [0u8; 32];
        e[0] = 0xC1;
        e[2..4].copy_from_slice(&(name.len() as u16).to_le_bytes());
        for (k, &ch) in name.iter().enumerate() {
            e[4 + k * 2] = ch;
            e[5 + k * 2] = 0;
        }
        BUF[slot * 32 + 64..slot * 32 + 96].copy_from_slice(&e);
        let dlba = HEAP + (CWD as u64 - ROOT_CLUSTER as u64) * SPC as u64;
        for s in 0..root_nsect() {
            let off = (s as usize) * 512;
            if Ahci::write_sector(dlba + s as u64, &BUF[off..off + 512].try_into().unwrap()) != 0 {
                return 1;
            }
        }
    }
    0
}

pub fn cwd() -> u32 {
    unsafe { CWD }
}

pub fn cd(name: &[u8]) -> u32 {
    unsafe {
        if name == b".." {
            CWD = ROOT_CLUSTER;
            return 0;
        }
        if name.len() == 0 || name.len() > 8 {
            return 1;
        }
        if mount() != 0 {
            return 1;
        }
        if read_dir() != 0 {
            return 1;
        }
        let idx = match entry_index(name) {
            Some(i) => i,
            None => return 1,
        };
        if idx < 2 {
            return 1;
        }
        let e83 = &BUF[(idx - 2) * 32..(idx - 2) * 32 + 32];
        if e83[0] != 0x83 || e83[3] & DIR_ATTR == 0 {
            return 1;
        }
        let c0 = &BUF[(idx - 1) * 32..(idx - 1) * 32 + 32];
        if c0[0] != 0xC0 {
            return 1;
        }
        CWD = rd32(c0, 21);
    }
    0
}
