static mut F_RET: u64 = 0;

#[cfg(test)]
static mut TEST_DISK: [u8; 512 * 2048] = [0; 512 * 2048];

#[cfg(not(test))]
fn raw_disk_read(lba: u64, count: u64, buf: u64) -> u64 {
    unsafe {
        core::arch::asm!(
            "mov r8, rsp",
            "syscall",
            inout("rax") 30u64 => _,
            in("rdi") lba,
            in("rsi") count,
            in("rdx") buf,
            out("r8") _,
            out("r11") _,
            options(nostack),
        );
    }
    0
}

#[cfg(not(test))]
fn raw_disk_write(lba: u64, count: u64, buf: u64) -> u64 {
    unsafe {
        core::arch::asm!(
            "mov r8, rsp",
            "syscall",
            inout("rax") 31u64 => _,
            in("rdi") lba,
            in("rsi") count,
            in("rdx") buf,
            out("r8") _,
            out("r11") _,
            options(nostack),
        );
    }
    0
}

#[cfg(test)]
fn raw_disk_read(lba: u64, count: u64, buf: u64) -> u64 {
    unsafe {
        let off = lba as usize * 512;
        let n = count as usize * 512;
        let src = TEST_DISK.as_ptr().add(off);
        core::ptr::copy_nonoverlapping(src, buf as *mut u8, n);
    }
    0
}

#[cfg(test)]
fn raw_disk_write(lba: u64, count: u64, buf: u64) -> u64 {
    unsafe {
        let off = lba as usize * 512;
        let n = count as usize * 512;
        let dst = TEST_DISK.as_mut_ptr().add(off);
        core::ptr::copy_nonoverlapping(buf as *const u8, dst, n);
    }
    0
}

pub const MAX_FILES: usize = 8;
pub const NAME_LEN: usize = 12;
const ENTRY_SIZE: usize = 40;
const ENTRIES_PER_BLOCK: usize = 512 / ENTRY_SIZE;
const ENTRY_START: usize = 12;
const ENTRY_SIZE_FIELD: usize = 20;
const ENTRY_BLOCKS: usize = 28;
const ENTRY_ATTR: usize = 32;
const ENTRY_MTIME: usize = 33;
const ENTRY_CTIME: usize = 35;
const ENTRY_OWNER: usize = 37;
const ENTRY_GROUP: usize = 38;
const ATTR_DIR: u8 = 1;
const ATTR_READONLY: u8 = 2;
const ATTR_HIDDEN: u8 = 4;
const ATTR_LINK: u8 = 8;
const MAX_PATH: usize = 8;
const MAX_PATH_LEN: usize = 128;

static mut CLOCK_MIN: u32 = 0;
static mut CURRENT_UID: u32 = 0;
static mut CURRENT_GID: u32 = 0;

static mut TRACK: [u8; 4096] = [0; 4096];

static mut CACHE_LBA: u64 = 0xFFFFFFFFFFFFFFFF;
static mut CACHE_BUF: [u8; 512] = [0; 512];

const EVENT_LOG_MAX: usize = 16;
static mut EVENT_LOG: [(u8, [u8; NAME_LEN]); EVENT_LOG_MAX] = [(0, [0; NAME_LEN]); EVENT_LOG_MAX];
static mut EVENT_HEAD: usize = 0;
static mut EVENT_COUNT: usize = 0;

fn event_log(kind: u8, name: &[u8]) {
    unsafe {
        let i = EVENT_HEAD % EVENT_LOG_MAX;
        EVENT_LOG[i].0 = kind;
        let mut n = [0u8; NAME_LEN];
        let k = core::cmp::min(name.len(), NAME_LEN);
        n[..k].copy_from_slice(&name[..k]);
        EVENT_LOG[i].1 = n;
        EVENT_HEAD += 1;
        if EVENT_COUNT < EVENT_LOG_MAX {
            EVENT_COUNT += 1;
        }
    }
}

pub fn event_count() -> usize {
    unsafe { EVENT_COUNT }
}

pub fn event_dump(out: &mut [u8]) -> usize {
    unsafe {
        let mut o = 0usize;
        let start = EVENT_HEAD.wrapping_sub(EVENT_COUNT);
        for k in 0..EVENT_COUNT {
            let i = (start + k) % EVENT_LOG_MAX;
            let (kind, name) = (EVENT_LOG[i].0, EVENT_LOG[i].1);
            let mark = match kind {
                1 => b'C',
                2 => b'W',
                3 => b'D',
                4 => b'T',
                5 => b'R',
                6 => b'L',
                7 => b'M',
                _ => b'?',
            };
            if o + 2 > out.len() {
                break;
            }
            out[o] = mark;
            out[o + 1] = b' ';
            o += 2;
            let mut nlen = 0usize;
            while nlen < NAME_LEN && name[nlen] != 0 {
                nlen += 1;
            }
            if o + nlen + 1 > out.len() {
                break;
            }
            out[o..o + nlen].copy_from_slice(&name[..nlen]);
            o += nlen;
            out[o] = b'\n';
            o += 1;
        }
        o
    }
}

pub fn gvfat_set_time(min: u32) {
    unsafe {
        CLOCK_MIN = min;
    }
}

fn clock_sec() -> u16 {
    unsafe { ((CLOCK_MIN as u64 * 60) / 2) as u16 }
}

pub struct File {
    pub valid: bool,
    pub name: [u8; NAME_LEN],
    pub path: [u8; MAX_PATH_LEN],
    pub start_block: u64,
    pub size: u64,
    pub pos: u64,
    pub blocks: u32,
    pub attr: u8,
    pub owner: u8,
    pub locked: bool,
}

const FILE_NONE: File = File {
    valid: false,
    name: [0; NAME_LEN],
    path: [0; MAX_PATH_LEN],
    start_block: 0,
    size: 0,
    pos: 0,
    blocks: 0,
    attr: 0,
    owner: 0,
    locked: false,
};

static mut FILES: [File; MAX_FILES] = [FILE_NONE; MAX_FILES];

static mut BM_START: u64 = 1;
static mut DIR_START: u64 = 0;
static mut DIR_BLOCKS: u64 = 32;
static mut DATA_START: u64 = 0;
static mut TOTAL_BLOCKS: u64 = 0;
static mut BASE_LBA: u64 = 0;

fn gdisk_read(lba: u64, count: u64, buf: u64) -> u64 {
    let base = unsafe { BASE_LBA };
    raw_disk_read(base + lba, count, buf)
}

fn gdisk_write(lba: u64, count: u64, buf: u64) -> u64 {
    let base = unsafe { BASE_LBA };
    raw_disk_write(base + lba, count, buf)
}

pub fn mount(base_lba: u64) -> u32 {
    unsafe {
        BASE_LBA = base_lba;
    }
    let mut sb = [0u8; 512];
    gdisk_read(0, 1, sb.as_mut_ptr() as u64);
    if &sb[0..3] != b"GvF" {
        return 1;
    }
    unsafe {
        BM_START = u64::from_le_bytes([sb[12], sb[13], sb[14], sb[15], sb[16], sb[17], sb[18], sb[19]]);
        DIR_START = u64::from_le_bytes([sb[20], sb[21], sb[22], sb[23], sb[24], sb[25], sb[26], sb[27]]);
        DIR_BLOCKS = u64::from_le_bytes([sb[28], sb[29], sb[30], sb[31], sb[32], sb[33], sb[34], sb[35]]);
        DATA_START = u64::from_le_bytes([sb[36], sb[37], sb[38], sb[39], sb[40], sb[41], sb[42], sb[43]]);
        TOTAL_BLOCKS = u64::from_le_bytes([sb[44], sb[45], sb[46], sb[47], sb[48], sb[49], sb[50], sb[51]]);
    }
    0
}

pub fn init() {
    let _ = mount(0);
}

fn bitmap_get(block: u64) -> bool {
    let bm = unsafe { BM_START };
    let mut buf = [0u8; 512];
    gdisk_read(bm + block / 4096, 1, buf.as_mut_ptr() as u64);
    let bit = block % 4096;
    (buf[(bit / 8) as usize] >> (bit % 8)) & 1 != 0
}

fn bitmap_set(block: u64, used: bool) {
    let bm = unsafe { BM_START };
    let idx = block / 4096;
    let mut buf = [0u8; 512];
    gdisk_read(bm + idx, 1, buf.as_mut_ptr() as u64);
    let bit = block % 4096;
    let byte = (bit / 8) as usize;
    if used {
        buf[byte] |= 1 << (bit % 8);
    } else {
        buf[byte] &= !(1 << (bit % 8));
    }
    gdisk_write(bm + idx, 1, buf.as_mut_ptr() as u64);
}

fn find_free_run(count: u64) -> u64 {
    let data_start = unsafe { DATA_START };
    let total = unsafe { TOTAL_BLOCKS };
    let mut run = 0u64;
    let mut start = 0u64;
    let mut b = data_start;
    while b < total {
        if !bitmap_get(b) {
            if run == 0 {
                start = b;
            }
            run += 1;
            if run >= count {
                return start;
            }
        } else {
            run = 0;
        }
        b += 1;
    }
    0
}

fn name_eq(entry: &[u8], name: &[u8]) -> bool {
    let mut i = 0;
    loop {
        let c = entry[i];
        let nc = if i < name.len() { name[i] } else { 0 };
        if c == 0 && nc == 0 {
            return true;
        }
        if c != nc {
            return false;
        }
        i += 1;
        if i >= NAME_LEN {
            return false;
        }
    }
}

fn read_dir_block(dirstart: u64, dirblocks: u64, s: u64, buf: &mut [u8; 512]) {
    if s < dirblocks {
        let lba = dirstart + s;
        unsafe {
            if CACHE_LBA == lba {
                *buf = CACHE_BUF;
            } else {
                gdisk_read(lba, 1, buf.as_mut_ptr() as u64);
                CACHE_LBA = lba;
                CACHE_BUF = *buf;
            }
        }
    }
}

fn write_dir_block(dirstart: u64, dirblocks: u64, s: u64, buf: &[u8; 512]) {
    if s < dirblocks {
        let lba = dirstart + s;
        gdisk_write(lba, 1, buf.as_ptr() as u64);
        unsafe {
            CACHE_LBA = lba;
            CACHE_BUF = *buf;
        }
    }
}

fn entry_attr(e: &[u8]) -> u8 {
    e[ENTRY_ATTR]
}

fn entry_fields(e: &[u8]) -> (u64, u64, u32, u8) {
    let start_block =
        u64::from_le_bytes([e[ENTRY_START], e[ENTRY_START + 1], e[ENTRY_START + 2], e[ENTRY_START + 3], e[ENTRY_START + 4], e[ENTRY_START + 5], e[ENTRY_START + 6], e[ENTRY_START + 7]]);
    let size =
        u64::from_le_bytes([e[ENTRY_SIZE_FIELD], e[ENTRY_SIZE_FIELD + 1], e[ENTRY_SIZE_FIELD + 2], e[ENTRY_SIZE_FIELD + 3], e[ENTRY_SIZE_FIELD + 4], e[ENTRY_SIZE_FIELD + 5], e[ENTRY_SIZE_FIELD + 6], e[ENTRY_SIZE_FIELD + 7]]);
    let blocks =
        u32::from_le_bytes([e[ENTRY_BLOCKS], e[ENTRY_BLOCKS + 1], e[ENTRY_BLOCKS + 2], e[ENTRY_BLOCKS + 3]]);
    (start_block, size, blocks, e[ENTRY_ATTR])
}

fn find_entry(dirstart: u64, dirblocks: u64, name: &[u8]) -> Option<(u64, u64, u32, u8, u64, u32)> {
    for s in 0..dirblocks {
        let mut buf = [0u8; 512];
        read_dir_block(dirstart, dirblocks, s, &mut buf);
        for e in 0..ENTRIES_PER_BLOCK {
            let off = e * ENTRY_SIZE;
            if buf[off] == 0 {
                return None;
            }
            if name_eq(&buf[off..off + NAME_LEN], name) {
                let (start, size, blocks, attr) = entry_fields(&buf[off..off + ENTRY_SIZE]);
                return Some((start, size, blocks, attr, dirstart + s, e as u32));
            }
        }
    }
    None
}

fn find_slot(dirstart: u64, dirblocks: u64) -> Option<(u64, u32)> {
    for s in 0..dirblocks {
        let mut buf = [0u8; 512];
        read_dir_block(dirstart, dirblocks, s, &mut buf);
        for e in 0..ENTRIES_PER_BLOCK {
            let off = e * ENTRY_SIZE;
            if buf[off] == 0 || buf[off] == 0xE5 {
                return Some((dirstart + s, e as u32));
            }
        }
    }
    None
}

fn write_entry(dirstart: u64, dirblocks: u64, lba: u64, slot: u32, name: &[u8], start_block: u64, size: u64, blocks: u32, attr: u8) {
    let mut buf = [0u8; 512];
    let s = lba - dirstart;
    read_dir_block(dirstart, dirblocks, s, &mut buf);
    let off = slot as usize * ENTRY_SIZE;
    buf[off..off + name.len()].copy_from_slice(name);
    buf[off + ENTRY_START..off + ENTRY_START + 8].copy_from_slice(&start_block.to_le_bytes());
    buf[off + ENTRY_SIZE_FIELD..off + ENTRY_SIZE_FIELD + 8].copy_from_slice(&size.to_le_bytes());
    let bl = blocks.to_le_bytes();
    buf[off + ENTRY_BLOCKS] = bl[0];
    buf[off + ENTRY_BLOCKS + 1] = bl[1];
    buf[off + ENTRY_BLOCKS + 2] = bl[2];
    buf[off + ENTRY_BLOCKS + 3] = bl[3];
    buf[off + ENTRY_ATTR] = attr;
    buf[off + ENTRY_OWNER] = unsafe { CURRENT_UID as u8 };
    buf[off + ENTRY_GROUP] = unsafe { CURRENT_GID as u8 };
    write_dir_block(dirstart, dirblocks, s, &buf);
}

pub fn gvfat_setuid(uid: u32) {
    unsafe {
        CURRENT_UID = uid;
    }
}

pub fn gvfat_setgid(gid: u32) {
    unsafe {
        CURRENT_GID = gid;
    }
}

pub fn gvfat_chown(path: &[u8], owner: u32, group: u32) -> u32 {
    if path.is_empty() || path.len() > MAX_PATH_LEN {
        return 1;
    }
    let mut parts = [&[][..]; MAX_PATH];
    let n = split_path(path, &mut parts);
    if n == 0 {
        return 1;
    }
    let (pdirstart, pdirblocks, _s, _z, _b, _a, lba, slot) = match resolve_path(&parts[..n]) {
        Some(x) => x,
        None => return 1,
    };
    let mut buf = [0u8; 512];
    let s = lba - pdirstart;
    read_dir_block(pdirstart, pdirblocks, s, &mut buf);
    let off = slot as usize * ENTRY_SIZE;
    buf[off + ENTRY_OWNER] = owner as u8;
    buf[off + ENTRY_GROUP] = group as u8;
    write_dir_block(pdirstart, pdirblocks, s, &buf);
    0
}

fn entry_owner(e: &[u8]) -> u8 {
    e[ENTRY_OWNER]
}

fn can_write(attr: u8, owner: u8) -> bool {
    unsafe {
        if CURRENT_UID == 0 {
            return true;
        }
        if attr & ATTR_READONLY != 0 {
            return false;
        }
        owner == CURRENT_UID as u8
    }
}

fn stamp_entry(dirstart: u64, lba: u64, slot: u32) {
    let mut buf = [0u8; 512];
    let s = lba - dirstart;
    read_dir_block(dirstart, 1, s, &mut buf);
    let off = slot as usize * ENTRY_SIZE;
    let t = clock_sec().to_le_bytes();
    buf[off + ENTRY_MTIME] = t[0];
    buf[off + ENTRY_MTIME + 1] = t[1];
    buf[off + ENTRY_CTIME] = t[0];
    buf[off + ENTRY_CTIME + 1] = t[1];
    write_dir_block(dirstart, 1, s, &buf);
}

fn touch_entry(dirstart: u64, lba: u64, slot: u32) {
    let mut buf = [0u8; 512];
    let s = lba - dirstart;
    read_dir_block(dirstart, 1, s, &mut buf);
    let off = slot as usize * ENTRY_SIZE;
    let t = clock_sec().to_le_bytes();
    buf[off + ENTRY_MTIME] = t[0];
    buf[off + ENTRY_MTIME + 1] = t[1];
    write_dir_block(dirstart, 1, s, &buf);
}

fn split_path<'a>(path: &'a [u8], parts: &mut [&'a [u8]; MAX_PATH]) -> usize {
    let mut n = 0usize;
    let mut i = 0usize;
    while i < path.len() && n < MAX_PATH {
        while i < path.len() && (path[i] == b'/' || path[i] == 0) {
            i += 1;
        }
        if i >= path.len() {
            break;
        }
        let start = i;
        while i < path.len() && path[i] != b'/' && path[i] != 0 {
            i += 1;
        }
        if i - start > NAME_LEN {
            return 0;
        }
        parts[n] = &path[start..i];
        n += 1;
    }
    n
}

fn resolve_dir<'a>(parts: &[&'a [u8]], depth: usize) -> Option<(u64, u64)> {
    let mut dirstart = unsafe { DIR_START };
    let mut dirblocks = unsafe { DIR_BLOCKS };
    let mut d = 0usize;
    while d + 1 < depth {
        let (start, _size, _blocks, attr, _lba, _slot) = find_entry(dirstart, dirblocks, parts[d])?;
        if attr & ATTR_DIR == 0 {
            return None;
        }
        dirstart = start;
        dirblocks = if _blocks > 0 { _blocks as u64 } else { 1 };
        d += 1;
    }
    Some((dirstart, dirblocks))
}

fn get_slot_addr(dirstart: u64, lba: u64, slot: u32) -> (u64, usize) {
    let mut buf = [0u8; 512];
    gdisk_read(lba, 1, buf.as_mut_ptr() as u64);
    let off = slot as usize * ENTRY_SIZE;
    if buf[off] == 0 || buf[off] == 0xE5 {
        (0, 0)
    } else {
        (u64::from_le_bytes([buf[off + ENTRY_START], buf[off + ENTRY_START + 1], buf[off + ENTRY_START + 2], buf[off + ENTRY_START + 3], buf[off + ENTRY_START + 4], buf[off + ENTRY_START + 5], buf[off + ENTRY_START + 6], buf[off + ENTRY_START + 7]]), off)
    }
}

fn resolve_path<'a>(parts: &[&'a [u8]]) -> Option<(u64, u64, u64, u64, u32, u8, u64, u32)> {
    let depth = parts.len();
    let (pdirstart, pdirblocks) = resolve_dir(parts, depth)?;
    let last = parts[depth - 1];
    let (start, size, blocks, attr, lba, slot) = find_entry(pdirstart, pdirblocks, last)?;
    Some((pdirstart, pdirblocks, start, size, blocks, attr, lba, slot))
}

pub fn gvfat_open(path: &[u8]) -> u32 {
    if path.is_empty() || path.len() > MAX_PATH_LEN {
        return 0xFFFFFFFF;
    }
    let mut parts = [&[][..]; MAX_PATH];
    let n = split_path(path, &mut parts);
    if n == 0 {
        return 0xFFFFFFFF;
    }
    if let Some((pdirstart, pdirblocks, start, size, blocks, attr, lba, slot)) = resolve_path_with_links(&parts[..n]) {
        if attr & ATTR_DIR != 0 {
            return 0xFFFFFFFF;
        }
        let mut obuf = [0u8; 512];
        gdisk_read(lba, 1, obuf.as_mut_ptr() as u64);
        let owner = obuf[slot as usize * ENTRY_SIZE + ENTRY_OWNER];
        unsafe {
            for i in 0..MAX_FILES {
                if !FILES[i].valid {
                    let mut nm = [0u8; NAME_LEN];
                    let name = parts[n - 1];
                    nm[..name.len()].copy_from_slice(name);
                    let mut pth = [0u8; MAX_PATH_LEN];
                    pth[..path.len()].copy_from_slice(path);
                    FILES[i] = File {
                        valid: true,
                        name: nm,
                        path: pth,
                        start_block: start,
                        size,
                        pos: 0,
                        blocks,
                        attr,
                        owner,
                        locked: false,
                    };
                    return i as u32;
                }
            }
        }
        return 0xFFFFFFFF;
    }
    let (pdirstart, pdirblocks) = match resolve_dir(&parts[..n], n) {
        Some(x) => x,
        None => return 0xFFFFFFFF,
    };
    let (lba, slot) = match find_slot(pdirstart, pdirblocks) {
        Some(x) => x,
        None => return 0xFFFFFFFF,
    };
    let name = parts[n - 1];
    write_entry(pdirstart, pdirblocks, lba, slot, name, 0, 0, 0, 0);
    stamp_entry(pdirstart, lba, slot);
    event_log(1, name);
    unsafe {
        for i in 0..MAX_FILES {
            if !FILES[i].valid {
                let mut nm = [0u8; NAME_LEN];
                nm[..name.len()].copy_from_slice(name);
                let mut pth = [0u8; MAX_PATH_LEN];
                pth[..path.len()].copy_from_slice(path);
                FILES[i] = File {
                    valid: true,
                    name: nm,
                    path: pth,
                    start_block: 0,
                    size: 0,
                    pos: 0,
                    blocks: 0,
                    attr: 0,
                    owner: unsafe { CURRENT_UID as u8 },
                    locked: false,
                };
                return i as u32;
            }
        }
    }
    0xFFFFFFFF
}

pub fn gvfat_read(handle: u32, buf: &mut [u8]) -> u32 {
    unsafe {
        let f = &mut FILES[handle as usize];
        if !f.valid || f.start_block == 0 || f.locked {
            return 0;
        }
        let mut total = 0;
        while total < buf.len() && f.pos < f.size {
            let block = f.start_block + f.pos / 512;
            let byte = (f.pos % 512) as usize;
            if byte == 0 {
                let remain = (f.size - f.pos) as usize;
                let want = core::cmp::min(buf.len() - total, remain);
                let nblocks = core::cmp::min(want / 512, 8);
                if nblocks > 0 {
                    let mut big = [0u8; 4096];
                    gdisk_read(block, nblocks as u64, big.as_mut_ptr() as u64);
                    let n = nblocks * 512;
                    buf[total..total + n].copy_from_slice(&big[..n]);
                    total += n;
                    f.pos += n as u64;
                    continue;
                }
            }
            let mut sec = [0u8; 512];
            gdisk_read(block, 1, sec.as_mut_ptr() as u64);
            let n = core::cmp::min(
                512 - byte,
                core::cmp::min(buf.len() - total, (f.size - f.pos) as usize),
            );
            buf[total..total + n].copy_from_slice(&sec[byte..byte + n]);
            total += n;
            f.pos += n as u64;
        }
        total as u32
    }
}

pub fn gvfat_write(handle: u32, buf: &[u8]) -> u32 {
    unsafe {
        let f = &mut FILES[handle as usize];
        if !f.valid || f.attr & ATTR_READONLY != 0 || !can_write(f.attr, f.owner) || f.locked {
            return 0;
        }
        let need_blocks = ((f.pos as usize + buf.len()) + 511) / 512;
        if need_blocks as u32 > f.blocks {
            let mut new_blocks = need_blocks as u32 + 8;
            let mut new_start = find_free_run(new_blocks as u64);
            if new_start == 0 {
                new_blocks = need_blocks as u32;
                new_start = find_free_run(new_blocks as u64);
            }
            if new_start == 0 {
                return 0;
            }
            if f.start_block != 0 && f.size > 0 {
                let old_blocks = f.blocks;
                for b in 0..old_blocks {
                    let mut sec = [0u8; 512];
                    gdisk_read(f.start_block + b as u64, 1, sec.as_mut_ptr() as u64);
                    gdisk_write(new_start + b as u64, 1, sec.as_mut_ptr() as u64);
                }
                for b in 0..old_blocks {
                    bitmap_set(f.start_block + b as u64, false);
                }
            }
            for b in 0..new_blocks {
                bitmap_set(new_start + b as u64, true);
            }
            f.start_block = new_start;
            f.blocks = new_blocks;
        }
        let mut total = 0;
        while total < buf.len() {
            let block = f.start_block + f.pos / 512;
            let byte = (f.pos % 512) as usize;
            let mut sec = [0u8; 512];
            gdisk_read(block, 1, sec.as_mut_ptr() as u64);
            let n = core::cmp::min(512 - byte, buf.len() - total);
            sec[byte..byte + n].copy_from_slice(&buf[total..total + n]);
            gdisk_write(block, 1, sec.as_mut_ptr() as u64);
            total += n;
            f.pos += n as u64;
        }
        if f.pos > f.size {
            f.size = f.pos;
        }
        if total > 0 {
            let mut parts = [&[][..]; MAX_PATH];
            let n = split_path(&f.path, &mut parts);
            if n > 0 {
                if let Some((pdirstart, pdirblocks, _s, _z, _b, _a, lba, slot)) = resolve_path(&parts[..n]) {
                    let name = parts[n - 1];
                    write_entry(pdirstart, pdirblocks, lba, slot, name, f.start_block, f.size, f.blocks, f.attr);
                    touch_entry(pdirstart, lba, slot);
                }
            }
        }
        total as u32
    }
}

pub fn gvfat_mkdir(path: &[u8]) -> u32 {
    if path.is_empty() || path.len() > MAX_PATH_LEN {
        return 1;
    }
    let mut parts = [&[][..]; MAX_PATH];
    let n = split_path(path, &mut parts);
    if n == 0 {
        return 1;
    }
    if resolve_path(&parts[..n]).is_some() {
        return 1;
    }
    let (pdirstart, pdirblocks) = match resolve_dir(&parts[..n], n) {
        Some(x) => x,
        None => return 1,
    };
    let (lba, slot) = match find_slot(pdirstart, pdirblocks) {
        Some(x) => x,
        None => return 1,
    };
    let start_block = find_free_run(1);
    if start_block == 0 {
        return 1;
    }
    let mut zero = [0u8; 512];
    gdisk_write(start_block, 1, zero.as_mut_ptr() as u64);
    bitmap_set(start_block, true);
    let name = parts[n - 1];
    write_entry(pdirstart, pdirblocks, lba, slot, name, start_block, 0, 1, ATTR_DIR);
    stamp_entry(pdirstart, lba, slot);
    event_log(7, name);
    0
}

pub fn gvfat_ln(link_path: &[u8], target: &[u8]) -> u32 {
    if link_path.is_empty()
        || link_path.len() > MAX_PATH_LEN
        || target.is_empty()
        || target.len() > 511
    {
        return 1;
    }
    let mut parts = [&[][..]; MAX_PATH];
    let n = split_path(link_path, &mut parts);
    if n == 0 {
        return 1;
    }
    if resolve_path(&parts[..n]).is_some() {
        return 1;
    }
    let (pdirstart, pdirblocks) = match resolve_dir(&parts[..n], n) {
        Some(x) => x,
        None => return 1,
    };
    let (lba, slot) = match find_slot(pdirstart, pdirblocks) {
        Some(x) => x,
        None => return 1,
    };
    let start_block = find_free_run(1);
    if start_block == 0 {
        return 1;
    }
    let mut tbuf = [0u8; 512];
    tbuf[..target.len()].copy_from_slice(target);
    gdisk_write(start_block, 1, tbuf.as_mut_ptr() as u64);
    bitmap_set(start_block, true);
    let name = parts[n - 1];
    write_entry(
        pdirstart,
        pdirblocks,
        lba,
        slot,
        name,
        start_block,
        target.len() as u64,
        1,
        ATTR_LINK,
    );
    stamp_entry(pdirstart, lba, slot);
    event_log(6, name);
    0
}

fn read_link_target(start_block: u64, size: u64, out: &mut [u8]) -> usize {
    let mut buf = [0u8; 512];
    gdisk_read(start_block, 1, buf.as_mut_ptr() as u64);
    let n = core::cmp::min(size as usize, out.len());
    out[..n].copy_from_slice(&buf[..n]);
    n
}

pub fn gvfat_link(link_path: &[u8], target_path: &[u8]) -> u32 {
    if link_path.is_empty()
        || link_path.len() > MAX_PATH_LEN
        || target_path.is_empty()
        || target_path.len() > MAX_PATH_LEN
    {
        return 1;
    }
    let mut tparts = [&[][..]; MAX_PATH];
    let tn = split_path(target_path, &mut tparts);
    if tn == 0 {
        return 1;
    }
    let (_, _, tstart, tsize, tblocks, tattr, _, _) =
        match resolve_path(&tparts[..tn]) {
            Some(x) => x,
            None => return 1,
        };
    if tattr & ATTR_DIR != 0 || tattr & ATTR_LINK != 0 {
        return 1;
    }
    if tstart == 0 {
        return 1;
    }
    let mut lparts = [&[][..]; MAX_PATH];
    let ln = split_path(link_path, &mut lparts);
    if ln == 0 {
        return 1;
    }
    if resolve_path(&lparts[..ln]).is_some() {
        return 1;
    }
    let (pdirstart, pdirblocks) = match resolve_dir(&lparts[..ln], ln) {
        Some(x) => x,
        None => return 1,
    };
    let (lba, slot) = match find_slot(pdirstart, pdirblocks) {
        Some(x) => x,
        None => return 1,
    };
    let name = lparts[ln - 1];
    write_entry(
        pdirstart,
        pdirblocks,
        lba,
        slot,
        name,
        tstart,
        tsize,
        tblocks,
        tattr,
    );
    stamp_entry(pdirstart, lba, slot);
    event_log(5, name);
    0
}

fn resolve_path_with_links<'a>(parts: &[&'a [u8]]) -> Option<(u64, u64, u64, u64, u32, u8, u64, u32)> {
    let mut cur_buf = [0u8; MAX_PATH_LEN];
    let mut cur_len = 0usize;
    for (i, p) in parts.iter().enumerate() {
        if i > 0 && cur_len > 0 && cur_len < MAX_PATH_LEN {
            cur_buf[cur_len] = b'/';
            cur_len += 1;
        }
        if cur_len + p.len() <= MAX_PATH_LEN {
            cur_buf[cur_len..cur_len + p.len()].copy_from_slice(p);
            cur_len += p.len();
        } else {
            return None;
        }
    }
    let mut hops = 0usize;
    loop {
        let mut np = [&[][..]; MAX_PATH];
        let nn = split_path(&cur_buf[..cur_len], &mut np);
        if nn == 0 {
            return None;
        }
        let (pdirstart, pdirblocks, start, size, blocks, attr, lba, slot) = resolve_path(&np[..nn])?;
        if attr & ATTR_LINK != 0 && hops < 4 {
            let mut tgt = [0u8; MAX_PATH_LEN];
            let tn = read_link_target(start, size, &mut tgt);
            if tn == 0 {
                return None;
            }
            cur_buf[..tn].copy_from_slice(&tgt[..tn]);
            cur_len = tn;
            hops += 1;
            continue;
        }
        return Some((pdirstart, pdirblocks, start, size, blocks, attr, lba, slot));
    }
}

pub fn gvfat_list(path: &[u8], out: &mut [u8]) -> u32 {
    let mut parts = [&[][..]; MAX_PATH];
    let n = split_path(path, &mut parts);
    let (dirstart, dirblocks) = if n == 0 {
        (unsafe { DIR_START }, unsafe { DIR_BLOCKS })
    } else {
        let (pd, pdb) = match resolve_dir(&parts[..n], n) {
            Some(x) => x,
            None => return 0,
        };
        let last = parts[n - 1];
        match find_entry(pd, pdb, last) {
            Some((start, _size, blocks, attr, _l, _s)) => {
                if attr & ATTR_DIR != 0 {
                    (start, if blocks > 0 { blocks as u64 } else { 1 })
                } else {
                    return 0;
                }
            }
            None => return 0,
        }
    };
    let mut o = 0usize;
    for s in 0..dirblocks {
        let mut buf = [0u8; 512];
        read_dir_block(dirstart, dirblocks, s, &mut buf);
        for e in 0..ENTRIES_PER_BLOCK {
            let off = e * ENTRY_SIZE;
            if buf[off] == 0 {
                return o as u32;
            }
            if buf[off] == 0xE5 {
                continue;
            }
            let mut nlen = 0usize;
            while nlen < NAME_LEN && buf[off + nlen] != 0 {
                nlen += 1;
            }
            let attr = buf[off + ENTRY_ATTR];
            let size = u64::from_le_bytes([buf[off + ENTRY_SIZE_FIELD], buf[off + ENTRY_SIZE_FIELD + 1], buf[off + ENTRY_SIZE_FIELD + 2], buf[off + ENTRY_SIZE_FIELD + 3], buf[off + ENTRY_SIZE_FIELD + 4], buf[off + ENTRY_SIZE_FIELD + 5], buf[off + ENTRY_SIZE_FIELD + 6], buf[off + ENTRY_SIZE_FIELD + 7]]);
            if o + nlen + 8 > out.len() {
                return o as u32;
            }
            let mark = if attr & ATTR_DIR != 0 { b'D' } else { b'F' };
            out[o] = mark;
            out[o + 1] = b' ';
            for k in 0..nlen {
                out[o + 2 + k] = buf[off + k];
            }
            o += 2 + nlen;
            out[o] = b' ';
            o += 1;
            let mut d = size;
            let mut tmp = [0u8; 10];
            let mut ti = 0usize;
            loop {
                tmp[ti] = b'0' + (d % 10) as u8;
                ti += 1;
                d /= 10;
                if d == 0 {
                    break;
                }
            }
            while ti > 0 {
                ti -= 1;
                out[o] = tmp[ti];
                o += 1;
            }
            out[o] = b'\n';
            o += 1;
        }
    }
    o as u32
}

pub fn gvfat_remove(path: &[u8]) -> u32 {
    if path.is_empty() || path.len() > MAX_PATH_LEN {
        return 1;
    }
    let mut parts = [&[][..]; MAX_PATH];
    let n = split_path(path, &mut parts);
    if n == 0 {
        return 1;
    }
    let (pdirstart, pdirblocks, start, _size, blocks, attr, lba, slot) = match resolve_path(&parts[..n]) {
        Some(x) => x,
        None => return 1,
    };
    if attr & ATTR_DIR != 0 {
        let mut empty = true;
        let mut b = 0u32;
        while b < blocks {
            let mut buf = [0u8; 512];
            gdisk_read(start + b as u64, 1, buf.as_mut_ptr() as u64);
            for e in 0..ENTRIES_PER_BLOCK {
                let off = e * ENTRY_SIZE;
                if buf[off] != 0 && buf[off] != 0xE5 {
                    empty = false;
                }
            }
            b += 1;
        }
        if !empty {
            return 1;
        }
    }
    for b in 0..blocks {
        bitmap_set(start + b as u64, false);
    }
    let mut buf = [0u8; 512];
    let s = lba - pdirstart;
    read_dir_block(pdirstart, pdirblocks, s, &mut buf);
    buf[slot as usize * ENTRY_SIZE] = 0xE5;
    write_dir_block(pdirstart, pdirblocks, s, &buf);
    event_log(3, parts[n - 1]);
    0
}

pub fn gvfat_close(handle: u32) {
    unsafe {
        if (handle as usize) < MAX_FILES {
            FILES[handle as usize] = FILE_NONE;
        }
    }
}

pub fn gvfat_lock(handle: u32) -> u32 {
    unsafe {
        if (handle as usize) >= MAX_FILES || !FILES[handle as usize].valid {
            return 1;
        }
        if FILES[handle as usize].locked {
            return 1;
        }
        FILES[handle as usize].locked = true;
    }
    0
}

pub fn gvfat_unlock(handle: u32) -> u32 {
    unsafe {
        if (handle as usize) >= MAX_FILES {
            return 1;
        }
        FILES[handle as usize].locked = false;
    }
    0
}

pub fn gvfat_append(handle: u32, buf: &[u8]) -> u32 {
    unsafe {
        let f = &mut FILES[handle as usize];
        if !f.valid {
            return 0;
        }
        f.pos = f.size;
    }
    gvfat_write(handle, buf)
}

pub fn gvfat_truncate(path: &[u8], new_size: u32) -> u32 {
    if path.is_empty() || path.len() > MAX_PATH_LEN {
        return 1;
    }
    let mut parts = [&[][..]; MAX_PATH];
    let n = split_path(path, &mut parts);
    if n == 0 {
        return 1;
    }
    let (pdirstart, pdirblocks, start, size, blocks, attr, lba, slot) = match resolve_path(&parts[..n]) {
        Some(x) => x,
        None => return 1,
    };
    if attr & ATTR_DIR != 0 {
        return 1;
    }
    if new_size as u64 >= size {
        return 0;
    }
    let need_blocks = ((new_size as usize + 511) / 512) as u32;
    let mut extra = 0u32;
    if need_blocks < blocks {
        extra = blocks - need_blocks;
    }
    for b in 0..extra {
        bitmap_set(start + (need_blocks + b) as u64, false);
    }
    let new_blocks = if need_blocks > 0 { need_blocks } else { 0 };
    let name = parts[n - 1];
    write_entry(pdirstart, pdirblocks, lba, slot, name, start, new_size as u64, new_blocks, attr);
    touch_entry(pdirstart, lba, slot);
    event_log(4, name);
    0
}

pub fn gvfat_stat(path: &[u8], out: &mut [u8]) -> u32 {
    if path.is_empty() || path.len() > MAX_PATH_LEN {
        return 1;
    }
    let mut parts = [&[][..]; MAX_PATH];
    let n = split_path(path, &mut parts);
    if n == 0 {
        return 1;
    }
    let (_pdirstart, _pdirblocks, _start, size, blocks, attr, _lba, _slot) = match resolve_path(&parts[..n]) {
        Some(x) => x,
        None => return 1,
    };
    let mut o = 0usize;
    let mark = if attr & ATTR_DIR != 0 { b'D' } else { b'F' };
    if o < out.len() {
        out[o] = mark;
        o += 1;
    }
    let mut d = size;
    let mut tmp = [0u8; 10];
    let mut ti = 0usize;
    loop {
        tmp[ti] = b'0' + (d % 10) as u8;
        ti += 1;
        d /= 10;
        if d == 0 {
            break;
        }
    }
    while ti > 0 {
        ti -= 1;
        if o < out.len() {
            out[o] = tmp[ti];
            o += 1;
        }
    }
    if o < out.len() {
        out[o] = b' ';
        o += 1;
    }
    d = blocks as u64;
    ti = 0;
    loop {
        tmp[ti] = b'0' + (d % 10) as u8;
        ti += 1;
        d /= 10;
        if d == 0 {
            break;
        }
    }
    while ti > 0 {
        ti -= 1;
        if o < out.len() {
            out[o] = tmp[ti];
            o += 1;
        }
    }
    o as u32
}

fn track_blocks(dirstart: u64, dirblocks: u64, depth: usize) {
    if depth > 8 {
        return;
    }
    for s in 0..dirblocks {
        let mut buf = [0u8; 512];
        read_dir_block(dirstart, dirblocks, s, &mut buf);
        for e in 0..ENTRIES_PER_BLOCK {
            let off = e * ENTRY_SIZE;
            if buf[off] == 0 {
                return;
            }
            if buf[off] == 0xE5 {
                continue;
            }
            let (start, _size, blocks, attr) = entry_fields(&buf[off..off + ENTRY_SIZE]);
            if start != 0 && blocks > 0 {
                for b in 0..blocks {
                    let idx = start + b as u64;
                    if (idx as usize) < unsafe { TRACK.len() } {
                        unsafe { TRACK[idx as usize] = 1; }
                    }
                }
            }
            if attr & ATTR_DIR != 0 && start != 0 {
                track_blocks(start, if blocks > 0 { blocks as u64 } else { 1 }, depth + 1);
            }
        }
    }
}

pub fn gvfat_fsck(out: &mut [u8]) -> u32 {
    unsafe {
        TRACK = [0; 4096];
    }
    let dir_start = unsafe { DIR_START };
    let dir_blocks = unsafe { DIR_BLOCKS };
    let data_start = unsafe { DATA_START };
    let total = unsafe { TOTAL_BLOCKS };
    track_blocks(dir_start, dir_blocks, 0);
    let mut orphan = 0u32;
    let mut lost = 0u32;
    let mut b = data_start;
    while b < total && (b as usize) < unsafe { TRACK.len() } {
        let used = bitmap_get(b);
        let refd = unsafe { TRACK[b as usize] != 0 };
        if used && !refd {
            orphan += 1;
        }
        if !used && refd {
            lost += 1;
        }
        b += 1;
    }
    let mut o = 0usize;
    let w = |s: &[u8], out: &mut [u8], o: &mut usize| {
        let mut i = 0;
        while i < s.len() && *o < out.len() {
            out[*o] = s[i];
            *o += 1;
            i += 1;
        }
    };
    w(b"fsck: orphan=", out, &mut o);
    let mut tmp = [0u8; 12];
    let mut ti = 0usize;
    let mut v = orphan;
    loop {
        tmp[ti] = b'0' + (v % 10) as u8;
        ti += 1;
        v /= 10;
        if v == 0 {
            break;
        }
    }
    while ti > 0 {
        ti -= 1;
        out[o] = tmp[ti];
        o += 1;
    }
    w(b" lost=", out, &mut o);
    ti = 0;
    v = lost;
    loop {
        tmp[ti] = b'0' + (v % 10) as u8;
        ti += 1;
        v /= 10;
        if v == 0 {
            break;
        }
    }
    while ti > 0 {
        ti -= 1;
        out[o] = tmp[ti];
        o += 1;
    }
    w(b"\r\n", out, &mut o);
    if orphan + lost == 0 {
        0
    } else {
        1
    }
}

pub fn gvfat_mark_bad(block: u64) {
    let total = unsafe { TOTAL_BLOCKS };
    if block < total {
        bitmap_set(block, true);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup() {
        unsafe {
            FILES = [FILE_NONE; MAX_FILES];
            TEST_DISK.fill(0);
            TEST_DISK[0..3].copy_from_slice(b"GvF");
            TEST_DISK[4..8].copy_from_slice(&1u32.to_le_bytes());
            TEST_DISK[8..12].copy_from_slice(&512u32.to_le_bytes());
            TEST_DISK[12..20].copy_from_slice(&1u64.to_le_bytes());
            TEST_DISK[20..28].copy_from_slice(&2u64.to_le_bytes());
            TEST_DISK[28..36].copy_from_slice(&32u64.to_le_bytes());
            TEST_DISK[36..44].copy_from_slice(&34u64.to_le_bytes());
            TEST_DISK[44..52].copy_from_slice(&1024u64.to_le_bytes());
            CACHE_LBA = 0xFFFFFFFFFFFFFFFF;
            CLOCK_MIN = 0;
        }
        init();
    }

    #[test]
    fn write_read_roundtrip() {
        setup();
        let h = gvfat_open(b"test.txt");
        assert_ne!(h, 0xFFFFFFFF);
        let w = gvfat_write(h, b"Hello GVF");
        assert_eq!(w, 9);
        gvfat_close(h);
        let h2 = gvfat_open(b"test.txt");
        assert_ne!(h2, 0xFFFFFFFF);
        let mut buf = [0u8; 32];
        let r = gvfat_read(h2, &mut buf);
        assert_eq!(r, 9);
        assert_eq!(&buf[..9], b"Hello GVF");
        gvfat_close(h2);
    }

    #[test]
    fn multi_file() {
        setup();
        let a = gvfat_open(b"a.txt");
        let b = gvfat_open(b"b.txt");
        assert_ne!(a, 0xFFFFFFFF);
        assert_ne!(b, 0xFFFFFFFF);
        assert_eq!(gvfat_write(a, b"AAA"), 3);
        assert_eq!(gvfat_write(b, b"BBBB"), 4);
        gvfat_close(a);
        gvfat_close(b);
        let a2 = gvfat_open(b"a.txt");
        let b2 = gvfat_open(b"b.txt");
        let mut ba = [0u8; 16];
        let mut bb = [0u8; 16];
        let ra = gvfat_read(a2, &mut ba);
        assert_eq!(ra, 3);
        assert_eq!(gvfat_read(b2, &mut bb), 4);
        assert_eq!(&ba[..3], b"AAA");
        assert_eq!(&bb[..4], b"BBBB");
        gvfat_close(a2);
        gvfat_close(b2);
    }

    #[test]
    fn large_file() {
        setup();
        let mut data = [0u8; 2000];
        for i in 0..2000 {
            data[i] = (i % 251) as u8;
        }
        let h = gvfat_open(b"big.bin");
        assert_eq!(gvfat_write(h, &data), 2000);
        gvfat_close(h);
        let h2 = gvfat_open(b"big.bin");
        let mut back = [0u8; 2000];
        assert_eq!(gvfat_read(h2, &mut back), 2000);
        assert_eq!(&back[..], &data[..]);
        gvfat_close(h2);
    }

    #[test]
    fn overwrite() {
        setup();
        let h = gvfat_open(b"o.txt");
        assert_eq!(gvfat_write(h, b"HELLO"), 5);
        gvfat_close(h);
        let h2 = gvfat_open(b"o.txt");
        assert_eq!(gvfat_write(h2, b"HI"), 2);
        gvfat_close(h2);
        let h3 = gvfat_open(b"o.txt");
        let mut b3 = [0u8; 8];
        assert_eq!(gvfat_read(h3, &mut b3), 5);
        assert_eq!(&b3[..5], b"HILLO");
        gvfat_close(h3);
    }

    #[test]
    fn extend_write() {
        setup();
        let h = gvfat_open(b"e.txt");
        assert_eq!(gvfat_write(h, b"12"), 2);
        assert_eq!(gvfat_write(h, b"345"), 3);
        gvfat_close(h);
        let h2 = gvfat_open(b"e.txt");
        let mut b = [0u8; 8];
        assert_eq!(gvfat_read(h2, &mut b), 5);
        assert_eq!(&b[..5], b"12345");
        gvfat_close(h2);
    }

    #[test]
    fn nested_dirs() {
        setup();
        assert_eq!(gvfat_mkdir(b"dir1"), 0);
        assert_eq!(gvfat_mkdir(b"dir1/sub"), 0);
        let h = gvfat_open(b"dir1/sub/file.txt");
        assert_ne!(h, 0xFFFFFFFF);
        assert_eq!(gvfat_write(h, b"NESTED"), 6);
        gvfat_close(h);
        let h2 = gvfat_open(b"dir1/sub/file.txt");
        assert_ne!(h2, 0xFFFFFFFF);
        let mut b = [0u8; 16];
        let r = gvfat_read(h2, &mut b);
        assert_eq!(r, 6);
        assert_eq!(&b[..6], b"NESTED");
        gvfat_close(h2);
    }

    #[test]
    fn list_and_remove() {
        setup();
        assert_eq!(gvfat_mkdir(b"d"), 0);
        let h = gvfat_open(b"d/f.txt");
        assert_eq!(gvfat_write(h, b"XY"), 2);
        gvfat_close(h);
        let mut out = [0u8; 256];
        let n = gvfat_list(b"d", &mut out);
        assert!(n > 0);
        let mut b = [0u8; 32];
        let h2 = gvfat_open(b"d/f.txt");
        assert_eq!(gvfat_read(h2, &mut b), 2);
        assert_eq!(&b[..2], b"XY");
        gvfat_close(h2);
        assert_eq!(gvfat_remove(b"d/f.txt"), 0);
        assert_eq!(gvfat_remove(b"d"), 0);
    }

    #[test]
    fn metadata_stamps() {
        setup();
        gvfat_set_time(120);
        let h = gvfat_open(b"m.txt");
        assert_ne!(h, 0xFFFFFFFF);
        assert_eq!(gvfat_write(h, b"DATA"), 4);
        gvfat_close(h);
        let mut buf = [0u8; 512];
        gdisk_read(unsafe { DIR_START } as u64, 1, buf.as_mut_ptr() as u64);
        let off = 0usize;
        let mtime0 = u16::from_le_bytes([buf[off + ENTRY_MTIME], buf[off + ENTRY_MTIME + 1]]);
        let ctime0 = u16::from_le_bytes([buf[off + ENTRY_CTIME], buf[off + ENTRY_CTIME + 1]]);
        assert_eq!(buf[off + ENTRY_ATTR], 0);
        assert_eq!(mtime0, 120 * 60 / 2);
        assert_eq!(ctime0, 120 * 60 / 2);
        gvfat_set_time(180);
        let h2 = gvfat_open(b"m.txt");
        assert_ne!(h2, 0xFFFFFFFF);
        assert_eq!(gvfat_write(h2, b"X"), 1);
        gvfat_close(h2);
        let mut buf2 = [0u8; 512];
        gdisk_read(unsafe { DIR_START } as u64, 1, buf2.as_mut_ptr() as u64);
        let mtime1 = u16::from_le_bytes([buf2[off + ENTRY_MTIME], buf2[off + ENTRY_MTIME + 1]]);
        let ctime1 = u16::from_le_bytes([buf2[off + ENTRY_CTIME], buf2[off + ENTRY_CTIME + 1]]);
        assert_eq!(mtime1, 180 * 60 / 2);
        assert_eq!(ctime1, 120 * 60 / 2);
    }

    #[test]
    fn fsck_detects() {
        setup();
        gvfat_set_time(60);
        let h = gvfat_open(b"f.txt");
        assert_eq!(gvfat_write(h, b"HELLO FS"), 8);
        gvfat_close(h);
        let mut out = [0u8; 64];
        let r = gvfat_fsck(&mut out);
        assert_eq!(r, 0);
        let mut b = [0u8; 32];
        let h2 = gvfat_open(b"f.txt");
        assert_eq!(gvfat_read(h2, &mut b), 8);
        gvfat_close(h2);
        bitmap_set(34, false);
        let mut out2 = [0u8; 64];
        let r2 = gvfat_fsck(&mut out2);
        assert_eq!(r2, 1);
        gvfat_mark_bad(35);
        let mut out3 = [0u8; 64];
        let r3 = gvfat_fsck(&mut out3);
        assert_ne!(r3, 0);
    }

    #[test]
    fn api_truncate_append_stat() {
        setup();
        gvfat_set_time(30);
        let h = gvfat_open(b"t.txt");
        assert_ne!(h, 0xFFFFFFFF);
        assert_eq!(gvfat_write(h, b"0123456789ABCDEF"), 16);
        gvfat_close(h);
        assert_eq!(gvfat_append(0, b"!!"), 0);
        let h2 = gvfat_open(b"t.txt");
        assert_ne!(h2, 0xFFFFFFFF);
        let mut big = [0u8; 64];
        assert_eq!(gvfat_append(h2, b"XY"), 2);
        gvfat_close(h2);
        let h3 = gvfat_open(b"t.txt");
        assert_ne!(h3, 0xFFFFFFFF);
        let mut b = [0u8; 32];
        let r = gvfat_read(h3, &mut b);
        assert_eq!(r, 18);
        assert_eq!(&b[..18], b"0123456789ABCDEFXY");
        gvfat_close(h3);
        assert_eq!(gvfat_truncate(b"t.txt", 8), 0);
        let h4 = gvfat_open(b"t.txt");
        assert_ne!(h4, 0xFFFFFFFF);
        let mut b2 = [0u8; 16];
        let r2 = gvfat_read(h4, &mut b2);
        assert_eq!(r2, 8);
        assert_eq!(&b2[..8], b"01234567");
        gvfat_close(h4);
        let mut st = [0u8; 32];
        let n = gvfat_stat(b"t.txt", &mut st);
        assert!(n > 0);
        assert_eq!(st[0], b'F');
    }

    #[test]
    fn multi_disk_mount() {
        setup();
        gvfat_set_time(30);
        let h = gvfat_open(b"disk0.txt");
        assert_eq!(gvfat_write(h, b"D0"), 2);
        gvfat_close(h);
        let mut sb1 = [0u8; 512];
        sb1[0..3].copy_from_slice(b"GvF");
        sb1[12..20].copy_from_slice(&1u64.to_le_bytes());
        sb1[20..28].copy_from_slice(&2u64.to_le_bytes());
        sb1[28..36].copy_from_slice(&32u64.to_le_bytes());
        sb1[36..44].copy_from_slice(&34u64.to_le_bytes());
        sb1[44..52].copy_from_slice(&1024u64.to_le_bytes());
        raw_disk_write(1024, 1, sb1.as_mut_ptr() as u64);
        assert_eq!(mount(1024), 0);
        let h2 = gvfat_open(b"disk1.txt");
        assert_ne!(h2, 0xFFFFFFFF);
        assert_eq!(gvfat_write(h2, b"D1"), 2);
        gvfat_close(h2);
        let h3 = gvfat_open(b"disk1.txt");
        assert_ne!(h3, 0xFFFFFFFF);
        let mut b = [0u8; 8];
        assert_eq!(gvfat_read(h3, &mut b), 2);
        assert_eq!(&b[..2], b"D1");
        gvfat_close(h3);
        assert_eq!(mount(0), 0);
        let h4 = gvfat_open(b"disk0.txt");
        assert_ne!(h4, 0xFFFFFFFF);
        let mut b2 = [0u8; 8];
        assert_eq!(gvfat_read(h4, &mut b2), 2);
        assert_eq!(&b2[..2], b"D0");
        gvfat_close(h4);
    }
}
