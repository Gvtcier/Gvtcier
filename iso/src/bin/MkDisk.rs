use std::fs;
use std::io::{Seek, SeekFrom, Write};

#[cfg(windows)]
use std::os::windows::io::AsRawHandle;

#[cfg(windows)]
#[link(name = "kernel32")]
extern "system" {
    fn DeviceIoControl(
        h_device: *mut core::ffi::c_void,
        dw_io_control_code: u32,
        lp_in_buffer: *mut core::ffi::c_void,
        n_in_buffer_size: u32,
        lp_out_buffer: *mut core::ffi::c_void,
        n_out_buffer_size: u32,
        lp_bytes_returned: *mut u32,
        lp_overlapped: *mut core::ffi::c_void,
    ) -> i32;
}

#[cfg(windows)]
fn set_sparse(f: &fs::File) {
    unsafe {
        let h = f.as_raw_handle() as *mut core::ffi::c_void;
        let mut ret = 0u32;
        DeviceIoControl(
            h,
            0x900C4,
            core::ptr::null_mut(),
            0,
            core::ptr::null_mut(),
            0,
            &mut ret,
            core::ptr::null_mut(),
        );
    }
}

#[cfg(not(windows))]
fn set_sparse(_f: &fs::File) {}

const BYTES_PER_SECTOR: u32 = 512;
const FAT_OFFSET: u32 = 12;
const ROOT_CLUSTER: u32 = 2;
const BITMAP_CLUSTER: u32 = 3;

fn layout(gb: u64) -> (u64, u32, u32, u64, u64) {
    if gb == 0 {
        return (32768, 8, 32, 44, 4090);
    }
    let sectors = gb * 1024 * 1024 * 1024 / 512;
    let spc: u32 = if gb >= 1024 {
        1024
    } else if gb >= 128 {
        512
    } else {
        128
    };
    let mut fat_len: u32 = 32;
    let mut heap: u64 = 12 + fat_len as u64;
    let mut clusters: u64 = 0;
    for _ in 0..8 {
        clusters = (sectors - heap) / spc as u64;
        let need = (clusters * 4 + 511) / 512;
        if need == fat_len as u64 {
            break;
        }
        fat_len = need as u32;
        heap = 12 + fat_len as u64;
    }
    (sectors, spc, fat_len, heap, clusters)
}

fn fill_vbr(img: &mut [u8], sectors: u64, spc_shift: u8, fat_len: u32, heap: u64, clusters: u64) {
    img[0..3].copy_from_slice(&[0xEB, 0x76, 0x90]);
    img[3..11].copy_from_slice(b"EXFAT   ");
    img[19..27].copy_from_slice(&sectors.to_le_bytes());
    img[27..31].copy_from_slice(&FAT_OFFSET.to_le_bytes());
    img[31..35].copy_from_slice(&fat_len.to_le_bytes());
    img[35..43].copy_from_slice(&heap.to_le_bytes());
    img[43..51].copy_from_slice(&clusters.to_le_bytes());
    img[51..55].copy_from_slice(&ROOT_CLUSTER.to_le_bytes());
    img[55] = 9;
    img[56] = spc_shift;
    img[57] = 1;
    img[58] = 0x80;
    img[457..459].copy_from_slice(&[0x55, 0xAA]);
}

fn fat_head() -> [u8; 64] {
    let mut h = [0u8; 64];
    h[0..4].copy_from_slice(&0xFFFFFFF8u32.to_le_bytes());
    for i in 1..7 {
        h[i * 4..i * 4 + 4].copy_from_slice(&0xFFFFFFFFu32.to_le_bytes());
    }
    h[28..32].copy_from_slice(&9u32.to_le_bytes());
    h[32..36].copy_from_slice(&10u32.to_le_bytes());
    h[36..40].copy_from_slice(&0xFFFFFFFFu32.to_le_bytes());
    h
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut gb: u64 = 0;
    if args.len() > 1 {
        gb = args[1].parse().unwrap_or(0);
    }
    let (sectors, spc, fat_len, heap, clusters) = layout(gb);
    let spc_shift = spc.trailing_zeros() as u8;

    let mut big = [0u8; 10240];
    for i in 0..big.len() {
        big[i] = (i % 251) as u8;
    }
    let files: [(&str, u32, &[u8]); 5] = [
        ("A.TXT", 4, b"alpha"),
        ("B.TXT", 5, b"beta"),
        ("C.TXT", 6, b"gamma"),
        ("TEST.TXT", 7, b"Gvtcier disk OK"),
        ("BIG.TXT", 8, &big),
    ];

    if gb == 0 {
        let mut img = vec![0u8; (sectors as u32 * BYTES_PER_SECTOR) as usize];
        fill_vbr(&mut img, sectors, spc_shift, fat_len, heap, clusters);
        let fo = (FAT_OFFSET * BYTES_PER_SECTOR) as usize;
        img[fo..fo + 64].copy_from_slice(&fat_head());
        let bm_off = ((heap + (BITMAP_CLUSTER - ROOT_CLUSTER) as u64 * spc as u64)
            * BYTES_PER_SECTOR as u64) as usize;
        img[bm_off] = 0xFC;
        img[bm_off + 1] = 0x07;
        let ro = ((heap + 0) * BYTES_PER_SECTOR as u64) as usize;
        let mut e = [0u8; 32];
        e[0] = 0x80;
        e[4..8].copy_from_slice(&BITMAP_CLUSTER.to_le_bytes());
        e[8..16].copy_from_slice(&512u64.to_le_bytes());
        img[ro..ro + 32].copy_from_slice(&e);
        e = [0xFFu8; 32];
        e[0] = 0x82;
        let label = "GvFAT".encode_utf16().collect::<Vec<u16>>();
        for (k, u) in label.iter().enumerate() {
            e[1 + k * 2] = *u as u8;
            e[2 + k * 2] = (u >> 8) as u8;
        }
        img[ro + 32..ro + 64].copy_from_slice(&e);
        let mut off = ro + 64;
        for (name, clu, data) in &files {
            let _ = clu;
            e = [0u8; 32];
            e[0] = 0x83;
            e[3..5].copy_from_slice(&0x20u16.to_le_bytes());
            img[off..off + 32].copy_from_slice(&e);
            e = [0u8; 32];
            e[0] = 0xC0;
            e[1] = 0x01;
            e[4] = name.len() as u8;
            e[9..17].copy_from_slice(&(data.len() as u64).to_le_bytes());
            e[21..25].copy_from_slice(&clu.to_le_bytes());
            e[25..32].copy_from_slice(&data.len().to_le_bytes()[0..7]);
            img[off + 32..off + 64].copy_from_slice(&e);
            e = [0u8; 32];
            e[0] = 0xC1;
            e[2..4].copy_from_slice(&(name.len() as u16).to_le_bytes());
            let nm = name.encode_utf16().collect::<Vec<u16>>();
            for (k, u) in nm.iter().enumerate() {
                e[4 + k * 2] = *u as u8;
                e[5 + k * 2] = (u >> 8) as u8;
            }
            img[off + 64..off + 96].copy_from_slice(&e);
            off += 96;
        }
        for (name, clu, data) in &files {
            let _ = name;
            let doff = ((heap + (*clu as u64 - ROOT_CLUSTER as u64) * spc as u64)
                * BYTES_PER_SECTOR as u64) as usize;
            img[doff..doff + data.len()].copy_from_slice(data);
        }
        fs::write("disk.img", &img).expect("write disk.img failed");
    } else {
        let mut f = fs::File::create("disk.img").expect("create disk.img failed");
        set_sparse(&f);
        let mut boot = [0u8; 512];
        fill_vbr(&mut boot, sectors, spc_shift, fat_len, heap, clusters);
        f.write_all(&boot).unwrap();
        let fo = (FAT_OFFSET as u64) * 512;
        f.seek(SeekFrom::Start(fo)).unwrap();
        f.write_all(&fat_head()).unwrap();
        let ro = (heap + 0) * 512;
        f.seek(SeekFrom::Start(ro)).unwrap();
        let mut e = [0u8; 32];
        e[0] = 0x80;
        e[4..8].copy_from_slice(&BITMAP_CLUSTER.to_le_bytes());
        e[8..16].copy_from_slice(&512u64.to_le_bytes());
        f.write_all(&e).unwrap();
        e = [0xFFu8; 32];
        e[0] = 0x82;
        let label = "GvFAT".encode_utf16().collect::<Vec<u16>>();
        for (k, u) in label.iter().enumerate() {
            e[1 + k * 2] = *u as u8;
            e[2 + k * 2] = (u >> 8) as u8;
        }
        f.write_all(&e).unwrap();
        for (name, clu, data) in &files {
            let _ = clu;
            e = [0u8; 32];
            e[0] = 0x83;
            e[3..5].copy_from_slice(&0x20u16.to_le_bytes());
            f.write_all(&e).unwrap();
            e = [0u8; 32];
            e[0] = 0xC0;
            e[1] = 0x01;
            e[4] = name.len() as u8;
            e[9..17].copy_from_slice(&(data.len() as u64).to_le_bytes());
            e[21..25].copy_from_slice(&clu.to_le_bytes());
            e[25..32].copy_from_slice(&data.len().to_le_bytes()[0..7]);
            f.write_all(&e).unwrap();
            e = [0u8; 32];
            e[0] = 0xC1;
            e[2..4].copy_from_slice(&(name.len() as u16).to_le_bytes());
            let nm = name.encode_utf16().collect::<Vec<u16>>();
            for (k, u) in nm.iter().enumerate() {
                e[4 + k * 2] = *u as u8;
                e[5 + k * 2] = (u >> 8) as u8;
            }
            f.write_all(&e).unwrap();
        }
        let bm_off = (heap + (BITMAP_CLUSTER - ROOT_CLUSTER) as u64 * spc as u64) * 512;
        f.seek(SeekFrom::Start(bm_off)).unwrap();
        f.write_all(&[0xFC, 0x07]).unwrap();
        for (name, clu, data) in &files {
            let _ = name;
            let doff = (heap + (*clu as u64 - ROOT_CLUSTER as u64) * spc as u64) * 512;
            f.seek(SeekFrom::Start(doff)).unwrap();
            f.write_all(data).unwrap();
        }
        f.set_len(sectors * 512).unwrap();
    }
    println!(
        "disk.img: {} sectors, {} clusters, {} files, spc={}",
        sectors,
        clusters,
        files.len(),
        spc
    );
}
