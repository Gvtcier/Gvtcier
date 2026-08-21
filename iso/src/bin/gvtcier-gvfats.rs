use std::env;
use std::fs;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("usage: gvtcier-gvfats <out.img> [blocks]");
        std::process::exit(1);
    }
    let total: u32 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(16384);
    let bm: u32 = total / 4096 + 1;
    let dn: u32 = 32;
    let data_start = 1 + bm + dn;

    let mut img = vec![0u8; total as usize * 512];
    img[0..3].copy_from_slice(b"GVF");
    img[4..8].copy_from_slice(&1u32.to_le_bytes());
    img[8..12].copy_from_slice(&512u32.to_le_bytes());
    img[12..16].copy_from_slice(&1u32.to_le_bytes());
    img[16..20].copy_from_slice(&(1 + bm).to_le_bytes());
    img[20..24].copy_from_slice(&dn.to_le_bytes());
    img[24..28].copy_from_slice(&data_start.to_le_bytes());
    img[28..32].copy_from_slice(&total.to_le_bytes());

    fs::write(&args[1], &img).expect("write img");
    println!(
        "gvfat img: {} blocks ({} bytes), bm={} dir={} data_start={}",
        total,
        img.len(),
        bm,
        dn,
        data_start
    );
}
