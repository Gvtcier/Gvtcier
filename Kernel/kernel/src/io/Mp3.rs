use alloc::vec::Vec;
use super::Flac::BitReader;
use super::Ogg::fast_cos;

pub struct Mp3Info {
    pub version: u8,
    pub sample_rate: u32,
    pub bitrate: u32,
    pub channels: u8,
    pub frame_size: usize,
}

pub struct Granule {
    pub part2_3_length: u16,
    pub big_values: u16,
    pub global_gain: u8,
    pub scalefac_compress: u8,
    pub table_select: [u8; 3],
    pub region0_count: u8,
    pub region1_count: u8,
    pub preflag: bool,
    pub scalefac_scale: bool,
    pub count1: bool,
}

pub struct SideInfo {
    pub main_data_begin: u16,
    pub scfsi: [bool; 4],
    pub granules: Vec<Granule>,
}

pub struct HuffmanEntry {
    pub len: u8,
    pub x: i16,
    pub y: i16,
}

pub struct HuffmanTable {
    pub entries: Vec<HuffmanEntry>,
}

pub fn huffman_decode(r: &mut BitReader, table: &HuffmanTable) -> (i16, i16) {
    let mut code = 0u32;
    for len in 1..=19u8 {
        code = (code << 1) | r.read(1);
        let mut idx = 0u32;
        for e in &table.entries {
            if e.len == len {
                if idx == code {
                    return (e.x, e.y);
                }
                idx += 1;
            }
        }
    }
    (0, 0)
}

pub fn fast_pow2(x: f32) -> f32 {
    let mut v = x;
    let mut r = 1.0f32;
    if v >= 1.0 {
        while v >= 1.0 {
            r *= 2.0;
            v -= 1.0;
        }
    } else if v < 0.0 {
        while v < 0.0 {
            r *= 0.5;
            v += 1.0;
        }
    }
    r * (1.0 + v * 0.693 + v * v * 0.240 + v * v * v * 0.0555)
}

pub fn fast_log2(x: f32) -> f32 {
    let bits = x.to_bits();
    let exp = ((bits >> 23) & 0xFF) as i32 - 127;
    let m = (bits & 0x7FFFFF) as f32 / 8388608.0;
    let v = m;
    exp as f32 + v - v * v / 2.0 + v * v * v / 3.0 - v * v * v * v / 4.0
}

pub fn fast_powf(x: f32, n: f32) -> f32 {
    if x <= 0.0 {
        return 0.0;
    }
    fast_pow2(n * fast_log2(x))
}

pub fn requantize(is: i32, global_gain: u8, scalefac: i32, scale: bool) -> f32 {
    let s = if is < 0 { -1.0 } else { 1.0 };
    let a = fast_powf(is.abs() as f32, 4.0 / 3.0);
    let g = (global_gain as f32) / 4.0 - (scalefac as f32) * if scale { 2.0 } else { 1.0 } - 210.0;
    s * a * fast_pow2(g)
}

pub fn decode_main_data(r: &mut BitReader, g: &Granule, table: &HuffmanTable) -> Vec<f32> {
    let mut is = Vec::new();
    let max = g.big_values as usize * 2 + 64;
    for _ in 0..max {
        let (x, y) = huffman_decode(r, table);
        is.push(requantize(x as i32, g.global_gain, 0, g.scalefac_scale));
        is.push(requantize(y as i32, g.global_gain, 0, g.scalefac_scale));
    }
    is
}

fn scalefac_lens(compress: u8) -> (u8, u8) {
    match compress as usize {
        0..=3 => (0, 0),
        4..=7 => (0, 1),
        8..=11 => (0, 2),
        12..=15 => (0, 3),
        16..=19 => (1, 1),
        20..=23 => (1, 2),
        24..=27 => (1, 3),
        28..=31 => (2, 1),
        32..=35 => (2, 2),
        36..=39 => (2, 3),
        40..=43 => (3, 1),
        44..=47 => (3, 2),
        48..=51 => (3, 3),
        52..=55 => (4, 2),
        56..=59 => (4, 3),
        _ => (5, 3),
    }
}

pub fn decode_scalefac(r: &mut BitReader, compress: u8, scfsi: &[bool; 4], scfsi_band: u8) -> Vec<u8> {
    let (slen1, slen2) = scalefac_lens(compress);
    let mut fac = Vec::new();
    for i in 0..21u8 {
        let len = if i < 6 || (i >= 12 && i < 18) { slen1 } else { slen2 };
        if len > 0 && !scfsi[scfsi_band as usize] {
            fac.push(r.read(len) as u8);
        } else {
            fac.push(0);
        }
    }
    fac
}

pub fn decode_frame(data: &[u8]) -> Option<(Vec<i16>, Mp3Info)> {
    let i = info(data)?;
    let si = side_info(data, i.channels, false)?;
    let table = HuffmanTable {
        entries: Vec::new(),
    };
    let mut r = BitReader::new(&data[4 + if i.channels == 1 { 17 } else { 32 }..]);
    let mut is = Vec::new();
    for g in &si.granules {
        let _sf = decode_scalefac(&mut r, g.scalefac_compress, &si.scfsi, 0);
        let mut s = decode_main_data(&mut r, g, &table);
        is.append(&mut s);
    }
    let n = 18usize;
    let mut freq = Vec::new();
    freq.resize(n / 2, 0.0f32);
    for k in 0..n / 2 {
        freq[k] = is.get(k).copied().unwrap_or(0.0);
    }
    let mut out = Vec::new();
    out.resize(n, 0.0f32);
    let inv_n = 2.0 / n as f32;
    for i in 0..n {
        let mut s = 0.0f32;
        for k in 0..n / 2 {
            let angle = core::f32::consts::PI / n as f32
                * (2.0 * (i as f32 + 0.5 + n as f32 / 4.0) * (k as f32 + 0.5));
            s += freq[k] * fast_cos(angle);
        }
        out[i] = s * inv_n;
    }
    let mut pcm = Vec::new();
    for s in &out {
        pcm.push((s.clamp(-1.0, 1.0) * 32767.0) as i16);
    }
    Some((pcm, i))
}

const BITRATES: [[u32; 15]; 2] = [
    [
        0, 32, 40, 48, 56, 64, 80, 96, 112, 128, 160, 192, 224, 256, 320,
    ],
    [
        0, 8, 16, 24, 32, 40, 48, 56, 64, 80, 96, 112, 128, 144, 160,
    ],
];

const SAMPLE_RATES: [[u32; 3]; 3] = [
    [44100, 48000, 32000],
    [22050, 24000, 16000],
    [11025, 12000, 8000],
];

pub fn info(data: &[u8]) -> Option<Mp3Info> {
    if data.len() < 4 {
        return None;
    }
    let h = u32::from_be_bytes([data[0], data[1], data[2], data[3]]);
    if h >> 21 != 0x7FF {
        return None;
    }
    let version = (h >> 19) & 0x3;
    let layer = (h >> 17) & 0x3;
    let bitrate_idx = ((h >> 12) & 0xF) as usize;
    let sr_idx = ((h >> 10) & 0x3) as usize;
    let padding = ((h >> 9) & 0x1) as usize;
    let mode = (h >> 6) & 0x3;
    if layer != 1 {
        return None;
    }
    let ver = match version {
        3 => 0,
        2 => 1,
        0 => 2,
        _ => return None,
    };
    let sr = SAMPLE_RATES[ver as usize][sr_idx];
    let br = BITRATES[if ver == 0 { 0usize } else { 1usize }][bitrate_idx] * 1000;
    let channels = if mode == 3 { 1 } else { 2 };
    let frame_size = if ver == 0 {
        (144 * br / sr) as usize + padding
    } else {
        (72 * br / sr) as usize + padding
    };
    if br == 0 || sr == 0 {
        return None;
    }
    Some(Mp3Info {
        version: ver,
        sample_rate: sr,
        bitrate: br,
        channels,
        frame_size,
    })
}

pub fn frames(data: &[u8]) -> Option<Vec<Mp3Info>> {
    let mut out = Vec::new();
    let mut off = 0usize;
    while off + 4 <= data.len() {
        if data[off] == 0xFF && data[off + 1] & 0xE0 == 0xE0 {
            if let Some(i) = info(&data[off..]) {
                let fs = i.frame_size;
                out.push(i);
                off += fs;
                continue;
            }
        }
        off += 1;
    }
    if out.is_empty() {
        None
    } else {
        Some(out)
    }
}

pub fn side_info(data: &[u8], channels: u8, protection: bool) -> Option<SideInfo> {
    let mut off = 4 + if protection { 2 } else { 0 };
    if off >= data.len() {
        return None;
    }
    let mut r = BitReader::new(&data[off..]);
    let main_data_begin = r.read(9) as u16;
    r.read(5);
    let mut scfsi = [false; 4];
    for i in 0..4 {
        scfsi[i] = r.read(1) == 1;
    }
    let mut granules = Vec::new();
    for _g in 0..2 {
        for _c in 0..channels as usize {
            let part2_3_length = r.read(12) as u16;
            let big_values = r.read(9) as u16;
            let global_gain = r.read(8) as u8;
            let scalefac_compress = r.read(4) as u8;
            let ws = r.read(1) == 1;
            let mut table_select = [0u8; 3];
            let mut region0_count = 0u8;
            let mut region1_count = 0u8;
            if ws {
                r.read(2);
                r.read(1);
                table_select[0] = r.read(5) as u8;
                table_select[1] = r.read(5) as u8;
                for _ in 0..3 {
                    r.read(3);
                }
            } else {
                table_select[0] = r.read(5) as u8;
                table_select[1] = r.read(5) as u8;
                table_select[2] = r.read(5) as u8;
                region0_count = r.read(5) as u8;
                region1_count = r.read(5) as u8;
            }
            let preflag = r.read(1) == 1;
            let scalefac_scale = r.read(1) == 1;
            let count1 = r.read(1) == 1;
            granules.push(Granule {
                part2_3_length,
                big_values,
                global_gain,
                scalefac_compress,
                table_select,
                region0_count,
                region1_count,
                preflag,
                scalefac_scale,
                count1,
            });
        }
    }
    Some(SideInfo {
        main_data_begin,
        scfsi,
        granules,
    })
}
